package main

import "encoding/json"

const schema = "newtui.behavior-corpus/v1"
const componentID = "newtui.example-dial/v1"
const artifactLimit = 8 * 1024 * 1024

var names = []string{"selection is always in range", "escape always closes without applying", "only adjustable rows move"}

type Input struct {
	Kind  string  `json:"kind"`
	Value *string `json:"value,omitzero"`
}
type Flow struct {
	Kind    string `json:"kind"`
	Applied *bool  `json:"applied,omitzero"`
}
type Outcome struct {
	Kind   string  `json:"kind"`
	Detail *string `json:"detail,omitzero"`
}
type Row struct {
	Label      string `json:"label"`
	Value      string `json:"value"`
	Note       string `json:"note"`
	Selected   bool   `json:"selected"`
	Adjustable bool   `json:"adjustable"`
}
type View struct {
	Title  string `json:"title"`
	Rows   []Row  `json:"rows"`
	Footer string `json:"footer"`
}
type Snapshot struct {
	View   View            `json:"view"`
	Intent json.RawMessage `json:"intent"`
}
type Step struct {
	Key   Input    `json:"key"`
	Flow  Flow     `json:"flow"`
	After Snapshot `json:"after"`
}
type Trace struct {
	Initial Snapshot `json:"initial"`
	Steps   []Step   `json:"steps"`
}
type CheckResult struct {
	Property uint64  `json:"property"`
	Outcome  Outcome `json:"outcome"`
}
type Event struct {
	Kind   string        `json:"kind"`
	Trace  Trace         `json:"trace"`
	Checks []CheckResult `json:"checks"`
}
type Domain struct {
	Alphabet  []Input `json:"alphabet"`
	MaxStates uint64  `json:"max_states"`
	MaxDepth  uint64  `json:"max_depth"`
}
type PropertyResult struct {
	Name         string  `json:"name"`
	Observations uint64  `json:"observations"`
	Applicable   uint64  `json:"applicable"`
	Held         uint64  `json:"held"`
	Outcome      Outcome `json:"outcome"`
}
type ReplayReason struct {
	Kind string  `json:"kind"`
	At   *uint64 `json:"at,omitzero"`
}
type ReplayIssue struct {
	Path   []Input      `json:"path"`
	Reason ReplayReason `json:"reason"`
}
type Exploration struct {
	States           uint64        `json:"states"`
	Transitions      uint64        `json:"transitions"`
	TerminalStates   uint64        `json:"terminal_states"`
	Exhausted        bool          `json:"exhausted"`
	Verdict          string        `json:"verdict"`
	IncompleteReason *string       `json:"incomplete_reason"`
	ReplayIssues     []ReplayIssue `json:"replay_issues"`
}
type Corpus struct {
	Schema      string           `json:"schema"`
	Component   string           `json:"component"`
	Seed        json.RawMessage  `json:"seed"`
	Domain      Domain           `json:"domain"`
	Exploration Exploration      `json:"exploration"`
	Properties  []PropertyResult `json:"properties"`
	Events      []Event          `json:"events"`
}
type Artifact struct {
	ID     string `json:"id"`
	Corpus Corpus `json:"corpus"`
}
