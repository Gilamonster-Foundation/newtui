use newtui::components::settings_panel::{
    acceptance, dial_values_are_accepted, Backend, Choice, Model, Setting, SettingChange,
    SettingsIntent, SettingsPanel, SettingsSeed,
};
use newtui::{Component, Explorer, Flow, Key, Observation, Property, PropertyOutcome, Row, View};

fn bounded_seed() -> SettingsSeed {
    SettingsSeed::new(
        vec![
            Setting::choice(
                "tenacity",
                "tenacity",
                "auto",
                vec![
                    Choice::new("auto", "inherit from the host"),
                    Choice::new("steady", "keep going"),
                    Choice::new("relentless", "do not stop"),
                ],
            ),
            Setting::number("rounds", "tool-call round limit", "auto", "auto", 1, 4),
            Setting::fixed(
                "prompt",
                "input prompt template",
                "❯ ",
                "edit with the host's text form",
            ),
        ],
        Model::new(
            "qwen",
            Some(vec![
                Choice::new("qwen", "active"),
                Choice::new("nemotron", "served"),
            ]),
        ),
        Backend::new(Some("sol")),
    )
}

// GUARD: bounded_settings_panel_is_exhaustively_clean — this is a guard; tests/mutations.rs must show it red.
#[test]
fn bounded_settings_panel_is_exhaustively_clean() {
    let seed = bounded_seed();
    let owned = acceptance(&seed);
    let properties: Vec<&dyn Property> = owned.iter().map(AsRef::as_ref).collect();
    let report =
        Explorer::new(Key::navigation()).explore(|| SettingsPanel::new(seed.clone()), &properties);

    assert!(report.exhausted, "the bounded walk must finish: {report}");
    assert!(report.is_clean(), "{report}");
}

#[test]
fn bounded_numbers_step_through_their_edges_and_report_changes() {
    let mut panel = SettingsPanel::new(bounded_seed());
    assert_eq!(panel.handle(Key::Down), Flow::Stay);
    assert_eq!(panel.view().rows[1].value, "auto");

    panel.handle(Key::Left);
    assert_eq!(panel.view().rows[1].value, "auto", "release clamps");
    panel.handle(Key::Right);
    assert_eq!(panel.view().rows[1].value, "1", "right enters at the floor");
    for _ in 0..8 {
        panel.handle(Key::Right);
    }
    assert_eq!(panel.view().rows[1].value, "4", "the ceiling clamps");
    assert_eq!(panel.handle(Key::Enter), Flow::Close(true));
    assert_eq!(
        panel.intent(),
        Some(&SettingsIntent::Apply {
            changes: vec![SettingChange {
                key: "rounds".to_string(),
                value: "4".to_string(),
            }],
            model: None,
        })
    );
}

#[test]
fn cancellation_never_returns_an_effect_intent() {
    let mut panel = SettingsPanel::new(bounded_seed());
    panel.handle(Key::Right);
    assert_eq!(panel.handle(Key::Esc), Flow::Close(false));
    assert_eq!(panel.intent(), None);

    let mut q = SettingsPanel::new(bounded_seed());
    assert_eq!(q.handle(Key::Char('q')), Flow::Close(false));
    let mut ctrl_q = SettingsPanel::new(bounded_seed());
    assert_eq!(ctrl_q.handle(Key::Ctrl('q')), Flow::Stay);
}

#[test]
fn fixed_values_and_the_backend_door_do_not_dial() {
    let mut panel = SettingsPanel::new(bounded_seed());
    panel.handle(Key::Down);
    panel.handle(Key::Down);
    let fixed = panel.view().rows[2].clone();
    assert!(!fixed.adjustable);
    assert!(fixed.note.contains("host's text form"), "{fixed:?}");
    panel.handle(Key::Right);
    assert_eq!(panel.view().rows[2].value, fixed.value);

    panel.handle(Key::Down);
    panel.handle(Key::Down);
    let door = panel.view().rows[4].clone();
    assert!(!door.adjustable);
    assert!(door.note.contains("Enter"), "{door:?}");
    panel.handle(Key::Left);
    panel.handle(Key::Right);
    assert_eq!(panel.view().rows[4].value, "sol");
    assert_eq!(panel.handle(Key::Enter), Flow::Close(true));
    assert_eq!(
        panel.intent(),
        Some(&SettingsIntent::OpenBackends {
            changes: Vec::new(),
            model: None,
        })
    );
}

#[test]
fn the_model_row_reports_a_pick_but_never_applies_it() {
    let mut panel = SettingsPanel::new(bounded_seed());
    for _ in 0..3 {
        panel.handle(Key::Down);
    }
    panel.handle(Key::Right);
    assert_eq!(panel.view().rows[3].value, "nemotron");
    assert_eq!(panel.handle(Key::Enter), Flow::Close(true));
    assert_eq!(
        panel.intent(),
        Some(&SettingsIntent::Apply {
            changes: Vec::new(),
            model: Some("nemotron".to_string()),
        })
    );
}

#[test]
fn an_unlistable_backend_keeps_the_active_model_and_explains_the_refusal() {
    let seed = SettingsSeed::new(
        Vec::new(),
        Model::new("still-active", None),
        Backend::new(None::<String>),
    );
    let mut panel = SettingsPanel::new(seed);
    let model = panel.view().rows[0].clone();
    assert_eq!(model.value, "still-active");
    assert!(!model.adjustable);
    assert!(model.note.contains("could not be listed"), "{model:?}");
    panel.handle(Key::Right);
    assert_eq!(panel.view().rows[0].value, "still-active");

    panel.handle(Key::Down);
    assert_eq!(panel.view().rows[1].value, "(none)");
    panel.handle(Key::Down);
    assert_eq!(panel.view().selected(), Some(1), "selection clamps");
}

#[test]
fn an_unserved_active_model_stays_the_opening_position() {
    let seed = SettingsSeed::new(
        Vec::new(),
        Model::new(
            "gone",
            Some(vec![Choice::new("a", "served"), Choice::new("b", "served")]),
        ),
        Backend::new(Some("sol")),
    );
    let mut panel = SettingsPanel::new(seed);
    assert_eq!(panel.view().rows[0].value, "gone");
    assert!(panel.view().rows[0].note.contains("not served"));
    assert_eq!(panel.handle(Key::Enter), Flow::Close(true));
    assert_eq!(
        panel.intent(),
        Some(&SettingsIntent::Apply {
            changes: Vec::new(),
            model: None,
        }),
        "opening the panel never silently changes the active model"
    );
}

#[test]
fn the_dial_property_declines_other_observations_and_refuses_an_unknown_value() {
    let seed = bounded_seed();
    let property = dial_values_are_accepted(&seed);
    let from = View::titled("settings").row(Row::new("tenacity", "auto").adjustable().selected());
    let invalid =
        View::titled("settings").row(Row::new("tenacity", "invented").adjustable().selected());
    assert_eq!(
        property.check(&Observation::State { view: &from }),
        PropertyOutcome::NotApplicable,
        "a state has no dial result to judge"
    );
    let outcome = property.check(&Observation::Transition {
        from: &from,
        key: Key::Right,
        to: &invalid,
        flow: Flow::Stay,
    });
    assert!(
        matches!(outcome, PropertyOutcome::Violated(ref detail) if detail.contains("refuses")),
        "{outcome:?}"
    );
}
