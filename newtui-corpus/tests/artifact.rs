use content_addressable::{canonical, ContentAddressable};
use newtui_corpus::model::*;

fn empty_walk() -> Corpus {
    Corpus {
        schema: SCHEMA.into(),
        component: "empty/v1".into(),
        seed: serde_json::json!({}),
        domain: Domain {
            alphabet: vec![],
            max_states: 10,
            max_depth: 4,
        },
        exploration: Exploration {
            states: 1,
            transitions: 0,
            terminal_states: 0,
            exhausted: true,
            verdict: Verdict::Incomplete,
            incomplete_reason: Some("NO PROPERTY WAS SUPPLIED — the walk judged nothing".into()),
            replay_issues: vec![],
        },
        properties: vec![],
        events: vec![Event {
            kind: EventKind::State,
            trace: Trace {
                initial: Snapshot {
                    view: View {
                        title: "empty".into(),
                        rows: vec![],
                        footer: "".into(),
                    },
                    intent: serde_json::Value::Null,
                },
                steps: vec![],
            },
            checks: vec![],
        }],
    }
}

fn forged(corpus: Corpus) -> Vec<u8> {
    // Recompute a valid content address: a domain failure must be rejected
    // independently of a mismatched hash.
    let id = corpus.content_id().unwrap().to_string();
    serde_json::to_vec(&Artifact { id, corpus }).unwrap()
}

#[test]
// GUARD: canonical_bytes_and_identity_are_owned_by_content_addressable
fn canonical_bytes_and_identity_are_owned_by_content_addressable() {
    let artifact = Artifact::mint(empty_walk()).unwrap();
    let encoded = artifact.canonical_bytes().unwrap();
    assert_eq!(
        encoded,
        canonical::to_canonical_dagcbor(&artifact.corpus).unwrap()
    );
    assert_eq!(
        Corpus::from_canonical_bytes(&encoded).unwrap(),
        artifact.corpus
    );
    assert_eq!(
        Artifact::from_json(&artifact.to_json().unwrap()).unwrap(),
        artifact
    );
    let mut altered = artifact.clone();
    altered.corpus.events[0].trace.initial.intent = serde_json::json!({"accepted": 3});
    assert!(
        Artifact::from_json(&serde_json::to_vec(&altered).unwrap()).is_err(),
        "an altered intent must break the content identity"
    );
    altered.id = "not-a-cid".into();
    assert!(Artifact::from_json(&serde_json::to_vec(&altered).unwrap()).is_err());
}

#[test]
fn a_valid_cid_does_not_excuse_a_malformed_domain_or_completeness_claim() {
    let valid = empty_walk();
    let mut mutations = Vec::new();
    let mut corpus = valid.clone();
    corpus.schema = "newtui.behavior-corpus/v2".into();
    mutations.push(corpus);
    let mut corpus = valid.clone();
    corpus.domain.max_depth = 65;
    mutations.push(corpus);
    let mut corpus = valid.clone();
    corpus.domain.alphabet = vec![Input::Right, Input::Right];
    mutations.push(corpus);
    let mut corpus = valid.clone();
    corpus.exploration.transitions = 1;
    mutations.push(corpus);
    let mut corpus = valid.clone();
    corpus.exploration.verdict = Verdict::Clean;
    corpus.exploration.incomplete_reason = None;
    mutations.push(corpus);
    let mut corpus = valid.clone();
    corpus.events.clear();
    mutations.push(corpus);
    for corpus in mutations {
        assert!(
            Artifact::from_json(&forged(corpus.clone())).is_err(),
            "accepted malformed record: {corpus:?}"
        );
        assert!(Artifact::mint(corpus).is_err());
    }
}

#[test]
fn portable_values_refuse_duplicate_keys_floats_and_lossy_integer_ranges() {
    let bytes = serde_json::to_string(&Artifact::mint(empty_walk()).unwrap()).unwrap();
    for seed in [
        "{\"x\":1,\"x\":2}",
        "{\"x\":1.0}",
        "{\"x\":9223372036854775808}",
    ] {
        let mutated = bytes.replace("\"seed\":{}", &format!("\"seed\":{seed}"));
        assert_ne!(mutated, bytes);
        assert!(Artifact::from_json(mutated.as_bytes()).is_err());
    }
    let mut corpus = empty_walk();
    corpus.seed = serde_json::json!({"x":1.5});
    assert!(Artifact::mint(corpus).is_err());
}

#[test]
fn checked_canonical_ingress_refuses_unknown_fields_instead_of_losing_them() {
    let original = empty_walk();
    let mut foreign = serde_json::to_value(&original).unwrap();
    foreign["unexpected"] = serde_json::json!(true);
    let bytes = canonical::to_canonical_dagcbor(&foreign).unwrap();
    assert!(Corpus::from_canonical_bytes(&bytes).is_err());
    let valid = original.canonical_form().unwrap();
    assert_eq!(Corpus::from_canonical_bytes(&valid).unwrap(), original);
}

#[test]
// GUARD: a_capped_verdict_cannot_excuse_edges_after_the_state_limit
fn a_capped_verdict_cannot_excuse_edges_after_the_state_limit() {
    let mut corpus = newtui_corpus::fixtures::dial().unwrap().corpus;
    corpus.domain.max_states = 4;
    corpus.exploration.exhausted = false;
    corpus.exploration.verdict = Verdict::Incomplete;
    corpus.exploration.incomplete_reason = Some("state cap reached".into());
    assert!(Artifact::from_json(&forged(corpus)).is_err());
}
