use std::cell::Cell;

use newtui::{properties, Component, Explorer, Flow, Key, Named, PropertyOutcome, Row, View};
use newtui_corpus::{capture, EventKind};

#[derive(Default)]
struct Dial {
    level: u8,
    intent: Option<u8>,
}

impl Component for Dial {
    fn handle(&mut self, key: Key) -> Flow {
        match key {
            Key::Right => self.level = (self.level + 1).min(3),
            Key::Left => self.level = self.level.saturating_sub(1),
            Key::Enter => {
                self.intent = Some(self.level);
                return Flow::Close(true);
            }
            Key::Esc => return Flow::Close(false),
            _ => {}
        }
        Flow::Stay
    }

    fn view(&self) -> View {
        View::titled("dial")
            .row(
                Row::new("level", self.level.to_string())
                    .selected()
                    .adjustable(),
            )
            .footer("arrows change")
    }
}

#[test]
fn tap_records_only_explored_observations_and_every_intermediate_result() {
    let explorer = Explorer::new(Key::navigation());
    let selection = properties::selection_is_always_in_range();
    let escape = properties::escape_always_closes_without_applying();
    let claims: [&dyn newtui::Property; 2] = [&selection, &escape];
    let expected = explorer.explore(Dial::default, &claims);
    let run = capture(&explorer, Dial::default, &claims, |dial| dial.intent);
    assert_eq!(run.report, expected);
    assert!(run.errors.is_empty(), "{:?}", run.errors);
    assert_eq!(run.report.states, 4);
    assert_eq!(run.report.transitions, 24);
    assert_eq!(run.events.len(), 36);
    assert_eq!(run.events[0].kind, EventKind::State);
    assert!(run.events[0].trace.steps.is_empty());
    let mut transitions = 0;
    for event in &run.events {
        let mut dial = Dial::default();
        assert_eq!(event.trace.initial.view, dial.view());
        assert_eq!(event.trace.initial.intent, None);
        for (index, step) in event.trace.steps.iter().enumerate() {
            assert_eq!(step.flow, dial.handle(step.key));
            assert_eq!(step.after.view, dial.view());
            assert_eq!(step.after.intent, dial.intent);
            if matches!(step.flow, Flow::Close(_)) {
                assert_eq!(index + 1, event.trace.steps.len());
            }
        }
        assert_eq!(event.checks.len(), 2);
        assert_eq!(event.checks[0].property, 0);
        assert_eq!(event.checks[1].property, 1);
        transitions += usize::from(event.kind == EventKind::Transition);
    }
    assert_eq!(transitions, run.report.transitions);
    assert!(run.events.iter().any(|event| {
        event
            .trace
            .steps
            .last()
            .is_some_and(|step| step.flow == Flow::Close(true) && step.after.intent == Some(3))
    }));
}

#[test]
fn recorder_does_not_change_empty_capped_or_inapplicable_verdicts() {
    let escape = properties::escape_always_closes_without_applying();
    for explorer in [
        Explorer::new([]),
        Explorer::new([Key::Right]),
        Explorer::new(Key::navigation()).max_states(1),
        Explorer::new(Key::navigation()).max_depth(0),
        Explorer::new(Key::navigation()).max_depth(1),
    ] {
        for claims in [vec![], vec![&escape as &dyn newtui::Property]] {
            let expected = explorer.explore(Dial::default, &claims);
            let run = capture(&explorer, Dial::default, &claims, |dial| dial.intent);
            assert_eq!(run.report, expected);
            assert_eq!(run.events[0].kind, EventKind::State);
            assert_eq!(
                run.events
                    .iter()
                    .filter(|event| event.kind == EventKind::Transition)
                    .count(),
                expected.transitions
            );
        }
    }
}

#[test]
// GUARD: duplicate_named_properties_keep_positions_and_retirement
fn duplicate_named_properties_keep_positions_and_retirement() {
    let first = Named::new(
        "portable observation recorder (not a property)",
        |_: &newtui::Observation<'_>| PropertyOutcome::Violated("first".into()),
    );
    let second = Named::new(
        "portable observation recorder (not a property)",
        |observation: &newtui::Observation<'_>| {
            if matches!(observation, newtui::Observation::Transition { .. }) {
                PropertyOutcome::Violated("second".into())
            } else {
                PropertyOutcome::NotApplicable
            }
        },
    );
    let explorer = Explorer::new(Key::navigation());
    let claims: [&dyn newtui::Property; 2] = [&first, &second];
    let run = capture(&explorer, Dial::default, &claims, |_| ());
    assert_eq!(run.report, explorer.explore(Dial::default, &claims));
    assert_eq!(run.events[0].checks.len(), 2);
    assert_eq!(run.events[1].checks.len(), 1);
    assert_eq!(run.events[1].checks[0].property, 1);
    assert!(run.events[2..].iter().all(|event| event.checks.is_empty()));
}

#[test]
fn replay_closing_early_never_creates_a_later_edge() {
    struct Unstable {
        dial: Dial,
        close: bool,
    }
    impl Component for Unstable {
        fn handle(&mut self, key: Key) -> Flow {
            if self.close {
                Flow::Close(false)
            } else {
                self.dial.handle(key)
            }
        }
        fn view(&self) -> View {
            self.dial.view()
        }
    }
    let builds = Cell::new(0);
    let factory = || {
        let count = builds.get();
        builds.set(count + 1);
        Unstable {
            dial: Dial::default(),
            close: count >= 5,
        }
    };
    let claim = properties::selection_is_always_in_range();
    let run = capture(
        &Explorer::new([Key::Right, Key::Left]),
        factory,
        &[&claim],
        |_| (),
    );
    assert!(!run.report.divergences().is_empty());
    assert_eq!(
        run.events
            .iter()
            .filter(|event| event.kind == EventKind::Transition)
            .count(),
        run.report.transitions
    );
    assert!(run.events.iter().all(|event| event
        .trace
        .steps
        .iter()
        .take(event.trace.steps.len().saturating_sub(1))
        .all(|step| step.flow == Flow::Stay)));
}

#[test]
fn each_trace_starts_from_the_actual_fresh_seed_including_its_intent() {
    let builds = Cell::new(0);
    let factory = || {
        let count = builds.get();
        builds.set(count + 1);
        Dial {
            level: 0,
            intent: (count > 0).then_some(9),
        }
    };
    let selection = properties::selection_is_always_in_range();
    let run = capture(
        &Explorer::new([Key::Enter]),
        factory,
        &[&selection],
        |dial| dial.intent,
    );
    // The hidden intent does not enter the component's default fingerprint.
    // The tap still refuses to describe different fresh starts as one seed.
    assert!(run.report.is_clean());
    assert_eq!(run.events[0].trace.initial.intent, None);
    assert_eq!(run.events[1].trace.initial.intent, Some(9));
    assert_eq!(
        run.errors,
        ["the factory produced a different observable initial state"]
    );
}

#[test]
fn an_impure_view_is_reported_instead_of_rewriting_the_captured_seed() {
    struct Impure(Cell<usize>);
    impl Component for Impure {
        fn handle(&mut self, _: Key) -> Flow {
            Flow::Stay
        }
        fn view(&self) -> View {
            let count = self.0.get();
            self.0.set(count + 1);
            View::titled(count.to_string())
        }
    }
    let run = capture(&Explorer::new([]), || Impure(Cell::new(0)), &[], |_| ());
    assert_eq!(run.events[0].trace.initial.view.title, "0");
    assert!(run
        .errors
        .iter()
        .any(|error| error.contains("without receiving a key")));
}
