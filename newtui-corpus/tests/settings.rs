use newtui::components::settings_panel::{
    Backend, Choice, Model, Setting, SettingsIntent, SettingsPanel, SettingsSeed,
};
use newtui::Property;
use newtui_corpus::{
    check, export, fixtures,
    model::{Domain, Input},
};
use serde_json::{json, Value};

fn seed() -> Value {
    json!({"mode":"quiet", "limit":"release", "label":"read only", "model":"small", "backend":"local"})
}

fn panel(seed: &Value) -> Result<SettingsPanel, String> {
    if seed != &self::seed() {
        return Err("this fixture requires its declared seed".into());
    }
    Ok(SettingsPanel::new(SettingsSeed::new(
        vec![
            Setting::choice(
                "mode",
                "Mode",
                "quiet",
                vec![
                    Choice::new("quiet", "less output"),
                    Choice::new("loud", "more output"),
                ],
            ),
            Setting::number("limit", "Limit", "release", "release", 1, 2),
            Setting::fixed("label", "Label", "read only", "host dialog"),
        ],
        Model::new(
            "small",
            Some(vec![
                Choice::new("small", "fast"),
                Choice::new("large", "thorough"),
            ]),
        ),
        Backend::new(Some("local")),
    )))
}

fn intent(panel: &SettingsPanel) -> Value {
    match panel.intent() {
        None => Value::Null,
        Some(SettingsIntent::Apply { changes, model }) => {
            json!({"kind":"apply", "changes":changes.iter().map(|c| json!({"key":c.key,"value":c.value})).collect::<Vec<_>>(), "model":model})
        }
        Some(SettingsIntent::OpenBackends { changes, model }) => {
            json!({"kind":"open_backends", "changes":changes.iter().map(|c| json!({"key":c.key,"value":c.value})).collect::<Vec<_>>(), "model":model})
        }
    }
}

#[test]
fn real_settings_choices_numbers_models_and_backend_intents_round_trip() {
    let claims = fixtures::claims();
    let claims: Vec<&dyn Property> = claims.iter().map(|p| p.as_ref()).collect();
    let artifact = export(
        "newtui.settings-fixture/v1",
        seed(),
        Domain {
            alphabet: vec![
                Input::Up,
                Input::Down,
                Input::Left,
                Input::Right,
                Input::Enter,
                Input::Esc,
            ],
            max_states: 512,
            max_depth: 32,
        },
        || panel(&seed()).unwrap(),
        &claims,
        intent,
    )
    .unwrap();
    assert!(artifact.corpus.exploration.exhausted);
    assert_eq!(
        artifact.corpus.exploration.verdict,
        newtui_corpus::model::Verdict::Clean
    );
    let intents: Vec<_> = artifact
        .corpus
        .events
        .iter()
        .flat_map(|e| &e.trace.steps)
        .map(|s| &s.after.intent)
        .collect();
    assert!(intents
        .iter()
        .any(|i| i["kind"] == "apply" && i["model"] == "large"));
    assert!(intents
        .iter()
        .any(|i| i["kind"] == "open_backends" && !i["changes"].as_array().unwrap().is_empty()));
    assert!(intents
        .iter()
        .any(|i| i["changes"].as_array().is_some_and(|changes| changes
            .iter()
            .any(|c| c["key"] == "limit" && c["value"] == "2"))));
    let loaded = newtui_corpus::model::Artifact::from_json(&artifact.to_json().unwrap()).unwrap();
    assert_eq!(
        check(&loaded, panel, &claims, intent).unwrap(),
        artifact.corpus.events.len()
    );
}
