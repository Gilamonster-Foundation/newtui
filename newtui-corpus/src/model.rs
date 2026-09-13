//! The portable application record. Content-addressable owns its encoding and CID.

use content_addressable::{canonical, ContentAddressable, ContentError};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const SCHEMA: &str = "newtui.behavior-corpus/v1";
pub const MAX_ARTIFACT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Input {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Backspace,
    Tab,
    BackTab,
    Home,
    End,
    PageUp,
    PageDown,
    Char(char),
    Ctrl(char),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Flow {
    Stay,
    Close { applied: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub label: String,
    pub value: String,
    pub note: String,
    pub selected: bool,
    pub adjustable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct View {
    pub title: String,
    pub rows: Vec<Row>,
    pub footer: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub view: View,
    #[serde(deserialize_with = "crate::value::deserialize")]
    pub intent: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Step {
    pub key: Input,
    pub flow: Flow,
    pub after: Snapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Trace {
    pub initial: Snapshot,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    State,
    Transition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Outcome {
    NotApplicable,
    Held,
    Violated { detail: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// Position in `Corpus::properties`; duplicate names are independent claims.
    pub property: u64,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Event {
    pub kind: EventKind,
    pub trace: Trace,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Domain {
    pub alphabet: Vec<Input>,
    pub max_states: u64,
    pub max_depth: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyResult {
    pub name: String,
    pub observations: u64,
    pub applicable: u64,
    pub held: u64,
    pub outcome: Outcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ReplayReason {
    DifferentState,
    ClosedDuringReplay { at: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayIssue {
    pub path: Vec<Input>,
    pub reason: ReplayReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Clean,
    Violated,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exploration {
    pub states: u64,
    pub transitions: u64,
    pub terminal_states: u64,
    pub exhausted: bool,
    pub verdict: Verdict,
    /// The core report's explanation, including an incomplete violated walk.
    pub incomplete_reason: Option<String>,
    /// Observable path/reason only; Rust-internal fingerprints are never exported.
    pub replay_issues: Vec<ReplayIssue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Corpus {
    pub schema: String,
    /// Versioned component contract understood by the consumer's factory.
    pub component: String,
    #[serde(deserialize_with = "crate::value::deserialize")]
    pub seed: Value,
    pub domain: Domain,
    pub exploration: Exploration,
    pub properties: Vec<PropertyResult>,
    pub events: Vec<Event>,
}

impl ContentAddressable for Corpus {
    fn canonical_form(&self) -> Result<Vec<u8>, ContentError> {
        canonical::to_canonical_dagcbor(self)
    }
}

/// JSON is only a transport. The id addresses the corpus's canonical DAG-CBOR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub id: String,
    pub corpus: Corpus,
}
