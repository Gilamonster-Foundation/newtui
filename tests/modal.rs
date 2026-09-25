use newtui::layout::{ModalSize, SizeKey, FILL, MIN_ROWS};
use newtui::{Component, Explorer, Flow, Key, Named, Observation, PropertyOutcome, Row, View};

/// A host with a `screen`-row terminal: it grants the request, clamped.
fn grant(request: u16, screen: u16) -> u16 {
    request.min(screen)
}

/// Drag targets below, at and above the floor, and past every screen tried.
const DRAGS: [(char, u16); 4] = [('0', 0), ('1', 1), ('4', MIN_ROWS), ('9', 20)];

/// A modal on a fixed screen, driven the way a host would drive it: every key
/// is decoded to an intent and applied against the height actually granted.
struct Host {
    size: ModalSize,
    screen: u16,
}

impl Component for Host {
    fn handle(&mut self, key: Key) -> Flow {
        let intent = match key {
            Key::Up => SizeKey::Grow,
            Key::Down => SizeKey::Shrink,
            Key::Char('z') => SizeKey::Zoom,
            Key::Char(c) => match DRAGS.iter().find(|(drag, _)| *drag == c) {
                Some((_, rows)) => SizeKey::To(*rows),
                None => return Flow::Stay,
            },
            _ => return Flow::Stay,
        };
        self.size
            .apply(intent, grant(self.size.requested(), self.screen));
        Flow::Stay
    }

    fn view(&self) -> View {
        View::titled("modal size")
            .row(Row::new("requested", self.size.requested().to_string()))
            .row(Row::new(
                "granted",
                grant(self.size.requested(), self.screen).to_string(),
            ))
            .row(Row::new("screen", self.screen.to_string()))
            .row(Row::new("zoomed", self.size.zoomed().to_string()))
            // What a second zoom restores: part of the state, so it is shown.
            .row(Row::new("restores", {
                let mut probe = self.size;
                probe.apply(SizeKey::Zoom, grant(probe.requested(), self.screen));
                if self.size.zoomed() {
                    probe.requested().to_string()
                } else {
                    "-".into()
                }
            }))
    }
}

fn number(view: &View, label: &str) -> u16 {
    view.rows
        .iter()
        .find(|row| row.label == label)
        .and_then(|row| row.value.parse().ok())
        .unwrap_or_else(|| panic!("the host view shows `{label}` as a number"))
}

/// Every reachable state, at every screen height 4..=12 and every starting
/// height 1..=14, under grow, shrink, zoom and four drag targets. The state
/// space is finite, so the walk exhausts it rather than sampling key sequences.
// GUARD: never_below_the_minimum_and_never_granted_past_the_screen — tests/mutations.rs must show it red.
#[test]
fn never_below_the_minimum_and_never_granted_past_the_screen() {
    let bounded = Named::new(
        "requested at least MIN_ROWS; granted at most the screen",
        |observation: &Observation<'_>| {
            let view = observation.view();
            let (requested, granted) = (number(view, "requested"), number(view, "granted"));
            if requested < MIN_ROWS {
                PropertyOutcome::Violated(format!("requested {requested} rows"))
            } else if granted > number(view, "screen") {
                PropertyOutcome::Violated(format!("granted {granted} rows"))
            } else {
                PropertyOutcome::Held
            }
        },
    );
    let steps = Named::new(
        "grow and shrink step one row from the granted height and leave zoom",
        |observation: &Observation<'_>| {
            let Observation::Transition { from, key, to, .. } = observation else {
                return PropertyOutcome::NotApplicable;
            };
            let granted = number(from, "granted");
            let expected = match key {
                Key::Up => granted.saturating_add(1).max(MIN_ROWS),
                Key::Down => granted.saturating_sub(1).max(MIN_ROWS),
                _ => return PropertyOutcome::NotApplicable,
            };
            let zoomed = to
                .rows
                .iter()
                .any(|row| row.label == "zoomed" && row.value == "true");
            if number(to, "requested") == expected && !zoomed {
                PropertyOutcome::Held
            } else {
                PropertyOutcome::Violated(format!("{key:?} from granted {granted}: {to:?}"))
            }
        },
    );
    let mut keys = vec![Key::Up, Key::Down, Key::Char('z'), Key::Other];
    keys.extend(DRAGS.iter().map(|(drag, _)| Key::Char(*drag)));
    for screen in MIN_ROWS..=12 {
        for start in 1..=14 {
            let report = Explorer::new(keys.clone()).explore(
                || Host {
                    size: ModalSize::new(start),
                    screen,
                },
                &[&bounded, &steps],
            );
            assert!(report.exhausted, "screen {screen}, start {start}: {report}");
            assert!(
                report.is_clean(),
                "screen {screen}, start {start}: {report}"
            );
        }
    }
}

// GUARD: zoom_fills_and_a_second_zoom_restores_the_prior_height — tests/mutations.rs must show it red.
#[test]
fn zoom_fills_and_a_second_zoom_restores_the_prior_height() {
    for screen in MIN_ROWS..=12 {
        for start in MIN_ROWS..=12 {
            let mut size = ModalSize::new(start);
            let before = grant(size.requested(), screen);
            assert_eq!(size.apply(SizeKey::Zoom, before), Some(FILL));
            assert!(size.zoomed());
            assert_eq!(
                grant(size.requested(), screen),
                screen,
                "zoom fills the screen"
            );
            size.apply(SizeKey::Zoom, screen);
            assert!(!size.zoomed());
            assert_eq!(grant(size.requested(), screen), before, "zoom round-trips");
        }
    }
}

// GUARD: grow_and_shrink_step_one_row_from_what_is_on_screen_and_leave_zoom — tests/mutations.rs must show it red.
#[test]
fn grow_and_shrink_step_one_row_from_what_is_on_screen_and_leave_zoom() {
    let mut size = ModalSize::new(8);
    assert_eq!(size.apply(SizeKey::Grow, 8), Some(9));
    assert_eq!(size.apply(SizeKey::Shrink, 9), Some(8));
    // Held at full height, Grow does not bank rows past what is granted.
    let mut full = ModalSize::new(8);
    full.apply(SizeKey::Grow, 12);
    full.apply(SizeKey::Grow, 12);
    assert_eq!(
        full.requested(),
        13,
        "steps from the granted 12, not the request"
    );
    assert_eq!(
        full.apply(SizeKey::Shrink, 12),
        Some(11),
        "one press shrinks one row"
    );
    // Sizing while zoomed leaves zoom.
    let mut zoomed = ModalSize::new(8);
    zoomed.apply(SizeKey::Zoom, 8);
    zoomed.apply(SizeKey::Shrink, 30);
    assert!(!zoomed.zoomed());
    assert_eq!(zoomed.requested(), 29);
}

// GUARD: a_drag_sets_the_exact_height_floored_and_leaves_zoom — tests/mutations.rs must show it red.
#[test]
fn a_drag_sets_the_exact_height_floored_and_leaves_zoom() {
    let mut size = ModalSize::new(8);
    assert_eq!(size.apply(SizeKey::To(11), 8), Some(11));
    assert_eq!(
        size.apply(SizeKey::To(11), 11),
        None,
        "no change, no request"
    );
    for rows in 0..=MIN_ROWS {
        assert_eq!(
            ModalSize::new(8).apply(SizeKey::To(rows), 8),
            Some(MIN_ROWS)
        );
    }
    // A drag while zoomed leaves zoom: a later zoom fills again, not restores.
    let mut zoomed = ModalSize::new(8);
    zoomed.apply(SizeKey::Zoom, 8);
    assert_eq!(zoomed.apply(SizeKey::To(6), 30), Some(6));
    assert!(!zoomed.zoomed());
    assert_eq!(zoomed.apply(SizeKey::Zoom, 6), Some(FILL));
}

// GUARD: a_shrink_at_the_minimum_changes_nothing — tests/mutations.rs must show it red.
#[test]
fn a_shrink_at_the_minimum_changes_nothing() {
    let mut size = ModalSize::new(MIN_ROWS);
    assert_eq!(size.apply(SizeKey::Shrink, MIN_ROWS), None);
    assert_eq!(size.requested(), MIN_ROWS);
}
