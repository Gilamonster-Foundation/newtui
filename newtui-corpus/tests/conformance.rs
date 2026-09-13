use newtui::{Component, Flow, Key, Property, View};
use newtui_corpus::{
    check, export, fixtures,
    model::{Domain, Input},
};
use serde_json::{json, Value};

#[test]
fn exported_dial_round_trips_and_a_rust_consumer_replays_the_complete_evidence() {
    let artifact = fixtures::dial().unwrap();
    assert_eq!(artifact.corpus.exploration.transitions, 24);
    let claims = fixtures::claims();
    let claims: Vec<&dyn Property> = claims.iter().map(|p| p.as_ref()).collect();
    assert_eq!(
        check(
            &artifact,
            fixtures::Dial::from_seed,
            &claims,
            fixtures::Dial::intent
        )
        .unwrap(),
        36
    );
    assert_eq!(
        newtui_corpus::model::Artifact::from_json(&artifact.to_json().unwrap()).unwrap(),
        artifact
    );
}

struct WrongFlow(fixtures::Dial);
impl Component for WrongFlow {
    fn handle(&mut self, key: Key) -> Flow {
        match self.0.handle(key) {
            Flow::Close(applied) => Flow::Close(!applied),
            flow => flow,
        }
    }
    fn view(&self) -> View {
        self.0.view()
    }
}

#[test]
// GUARD: the_same_reachable_views_do_not_excuse_different_flows_or_intents
fn the_same_reachable_views_do_not_excuse_different_flows_or_intents() {
    let artifact = fixtures::dial().unwrap();
    let claims = fixtures::claims();
    let claims: Vec<&dyn Property> = claims.iter().map(|p| p.as_ref()).collect();
    let wrong = check(
        &artifact,
        |seed| fixtures::Dial::from_seed(seed).map(WrongFlow),
        &claims,
        |dial| dial.0.intent(),
    )
    .unwrap_err();
    assert!(wrong.contains("flow"), "{wrong}");
    let wrong = check(&artifact, fixtures::Dial::from_seed, &claims, |_| {
        Value::Null
    })
    .unwrap_err();
    assert!(wrong.contains("intent"), "{wrong}");
    let wrong = check(
        &artifact,
        |_| fixtures::Dial::from_seed(&json!({"level":1,"maximum":3})),
        &claims,
        fixtures::Dial::intent,
    )
    .unwrap_err();
    assert!(wrong.contains("initial"), "{wrong}");
}

#[test]
fn a_matching_capped_walk_never_passes_conformance() {
    let seed = json!({"level":0,"maximum":3});
    let claims = fixtures::claims();
    let claims: Vec<&dyn Property> = claims.iter().map(|p| p.as_ref()).collect();
    let artifact = export(
        fixtures::DIAL,
        seed.clone(),
        Domain {
            alphabet: vec![Input::Left, Input::Right, Input::Enter, Input::Esc],
            max_states: 2,
            max_depth: 8,
        },
        || fixtures::Dial::from_seed(&seed).unwrap(),
        &claims,
        fixtures::Dial::intent,
    )
    .unwrap();
    assert!(!artifact.corpus.exploration.exhausted);
    let error = check(
        &artifact,
        fixtures::Dial::from_seed,
        &claims,
        fixtures::Dial::intent,
    )
    .unwrap_err();
    assert!(error.contains("incomplete"), "{error}");
}

#[test]
fn empty_alphabets_preserve_the_core_state_limit_and_depth_verdicts() {
    let seed = json!({"level": 0, "maximum": 3});
    let claims = fixtures::claims();
    let claims: Vec<&dyn Property> = claims.iter().map(|p| p.as_ref()).collect();
    for max_states in [0, 1, 2] {
        for max_depth in [0, 1, 8] {
            let report = newtui::Explorer::new([])
                .max_states(max_states)
                .max_depth(max_depth)
                .explore(|| fixtures::Dial::from_seed(&seed).unwrap(), &claims);
            let artifact = export(
                fixtures::DIAL,
                seed.clone(),
                Domain {
                    alphabet: vec![],
                    max_states: max_states as u64,
                    max_depth: max_depth as u64,
                },
                || fixtures::Dial::from_seed(&seed).unwrap(),
                &claims,
                fixtures::Dial::intent,
            )
            .unwrap();
            assert_eq!(artifact.corpus.exploration.exhausted, report.exhausted);
            assert_eq!(report.exhausted, max_states > 1 && max_depth > 0);
        }
    }
}
