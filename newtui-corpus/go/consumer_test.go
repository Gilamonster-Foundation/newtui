package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"testing"
)

func fixture(t *testing.T) []byte {
	t.Helper()
	raw, err := os.ReadFile("../fixtures/dial.json")
	if err != nil {
		t.Fatal(err)
	}
	return raw
}

func TestNativeConsumerAndCanonicalParity(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	created := 0
	n, err := Check(a, func(seed json.RawMessage) (Component, error) { created++; return NewDial(seed) })
	if err != nil || n != 36 || created != 36 {
		t.Fatalf("checked=%d created=%d err=%v", n, created, err)
	}
	encoded, err := Canonical(a.Corpus)
	if err != nil {
		t.Fatal(err)
	}
	golden, err := os.ReadFile("../fixtures/dial.cbor")
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(encoded, golden) {
		t.Fatal("official Go DAG-CBOR differs from Rust golden bytes")
	}
}

type wrongFlow struct{ Component }

func (d wrongFlow) Handle(key Input) Flow {
	f := d.Component.Handle(key)
	if f.Kind == "close" {
		value := !*f.Applied
		f.Applied = &value
	}
	return f
}

type wrongIntent struct{ Component }

func (d wrongIntent) Intent() json.RawMessage { return json.RawMessage("null") }

func TestNativeWrongConsumersAreRejected(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	for _, field := range []string{"flow", "intent"} {
		_, err = Check(a, func(seed json.RawMessage) (Component, error) {
			d, e := NewDial(seed)
			if field == "flow" {
				return wrongFlow{d}, e
			}
			return wrongIntent{d}, e
		})
		if err == nil || !strings.Contains(err.Error(), field) {
			t.Fatalf("wrong %s accepted: %v", field, err)
		}
	}
}

func TestCheckedIngressRejectsTamperingAndMalformedDomains(t *testing.T) {
	raw := fixture(t)
	for _, change := range []func([]byte) []byte{
		func(b []byte) []byte { return bytes.Replace(b, []byte(`"maximum": 3`), []byte(`"maximum": 4`), 1) },
		func(b []byte) []byte {
			return bytes.Replace(b, []byte(`"level": 0`), []byte(`"level": 0, "level": 1`), 1)
		},
		func(b []byte) []byte { return bytes.Replace(b, []byte(`"level": 0`), []byte(`"level": 0.0`), 1) },
		func(b []byte) []byte {
			return bytes.Replace(b, []byte(`"level": 0`), []byte(`"level": 9223372036854775808`), 1)
		},
	} {
		changed := change(raw)
		if bytes.Equal(changed, raw) {
			t.Fatal("test changed nothing")
		}
		if _, err := ReadArtifact(changed); err == nil {
			t.Fatal("invalid JSON/content accepted")
		}
	}
	for _, change := range []func(*Artifact){
		func(a *Artifact) { a.Corpus.Schema = "newtui.behavior-corpus/v99" },
		func(a *Artifact) { a.Corpus.Domain.Alphabet = []Input{{Kind: "left"}, {Kind: "left"}} },
		func(a *Artifact) { a.Corpus.Exploration.Transitions = 1 },
		func(a *Artifact) { a.Corpus.Events = nil },
	} {
		a, err := ReadArtifact(raw)
		if err != nil {
			t.Fatal(err)
		}
		change(a)
		a.ID, err = Identity(a.Corpus)
		if err != nil {
			t.Fatal(err)
		}
		changed, err := json.Marshal(a)
		if err != nil {
			t.Fatal(err)
		}
		if _, err := ReadArtifact(changed); err == nil {
			t.Fatal("valid CID excused malformed evidence")
		}
	}
}

func TestPortableUnicodeNumbersAndDepth(t *testing.T) {
	for _, raw := range []string{`"\ud800"`, `"\udc00"`, `1.0`, `9223372036854775808`, `{"x":1,"x":2}`, strings.Repeat("[", 33) + "0" + strings.Repeat("]", 33)} {
		if err := rawPortable(json.RawMessage(raw)); err == nil {
			t.Errorf("accepted %q", raw)
		}
	}
	for _, raw := range []string{`"\ud83d\ude42"`, `"é"`, `-9223372036854775808`, `9223372036854775807`, `{"/":"ordinary map"}`, strings.Repeat("[", 32) + "0" + strings.Repeat("]", 32)} {
		if err := rawPortable(json.RawMessage(raw)); err != nil {
			t.Errorf("rejected %q: %v", raw, err)
		}
	}
}

func TestStateCapCannotHideLaterEdges(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	a.Corpus.Domain.MaxStates = 4
	a.Corpus.Exploration.Exhausted = false
	a.Corpus.Exploration.Verdict = "incomplete"
	reason := "state cap reached"
	a.Corpus.Exploration.IncompleteReason = &reason
	a.ID, err = Identity(a.Corpus)
	if err != nil {
		t.Fatal(err)
	}
	raw, err := json.Marshal(a)
	if err != nil {
		t.Fatal(err)
	}
	if _, err := ReadArtifact(raw); err == nil {
		t.Fatal("accepted edges after cap")
	}
}

func TestCLIAndPropertyJudgments(t *testing.T) {
	var out bytes.Buffer
	if err := run([]string{"../fixtures/dial.json"}, &out); err != nil || !strings.Contains(out.String(), "36 observations") {
		t.Fatalf("%s %v", out.String(), err)
	}
	for _, args := range [][]string{nil, {"/no-such-newtui-corpus"}} {
		if err := run(args, &out); err == nil {
			t.Fatal("bad CLI usage passed")
		}
	}
	view := View{Rows: []Row{{Label: "fixed", Value: "old", Selected: true}}}
	after := View{Rows: []Row{{Label: "fixed", Value: "new", Selected: true}}}
	for _, tc := range []struct {
		name, kind, key string
		before, after   View
		flow            Flow
		want            string
	}{
		{names[0], "state", "other", view, View{Rows: []Row{}}, Flow{}, "not_applicable"},
		{names[0], "state", "other", view, View{Rows: []Row{{}}}, Flow{}, "violated"},
		{names[0], "state", "other", view, View{Rows: []Row{{Selected: true}, {Selected: true}}}, Flow{}, "violated"},
		{names[1], "transition", "esc", view, view, Flow{Kind: "stay"}, "violated"},
		{names[1], "transition", "esc", view, view, Flow{Kind: "close"}, "violated"},
		{names[2], "transition", "right", view, after, Flow{}, "violated"},
		{names[2], "transition", "left", view, after, Flow{}, "violated"},
		{names[2], "transition", "left", View{}, after, Flow{}, "not_applicable"},
		{names[2], "transition", "left", view, View{}, Flow{}, "not_applicable"},
	} {
		actual, err := judge(tc.name, tc.kind, tc.before, Input{Kind: tc.key}, tc.after, tc.flow)
		if err != nil || actual.Kind != tc.want {
			t.Fatalf("%+v: %v %v", tc, actual, err)
		}
	}
	if _, err := judge("unknown", "state", view, Input{}, view, Flow{}); err == nil {
		t.Fatal("unknown property accepted")
	}
	for _, seed := range []string{`{}`, `{"level":4,"maximum":3}`, `{"level":0,"maximum":21}`, `{"level":0,"maximum":3,"extra":1}`} {
		if _, err := NewDial(json.RawMessage(seed)); err == nil {
			t.Fatal("malformed seed accepted")
		}
	}
}

type mutatingInput struct{ Component }

func (d mutatingInput) Handle(k Input) Flow {
	f := d.Component.Handle(k)
	if k.Value != nil {
		*k.Value = "modified"
	}
	return f
}

func TestInputStorageCannotBeSharedWithTheConsumer(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	// Up is a no-op for the dial. Give that edge a character payload so a Go
	// consumer receives a pointer and the guard is exercised on every replay.
	value := "x"
	for i, k := range a.Corpus.Domain.Alphabet {
		if k.Kind == "up" {
			a.Corpus.Domain.Alphabet[i] = Input{Kind: "char", Value: &value}
		}
	}
	for i := range a.Corpus.Events {
		for j := range a.Corpus.Events[i].Trace.Steps {
			s := &a.Corpus.Events[i].Trace.Steps[j]
			if s.Key.Kind == "up" {
				v := "x"
				s.Key = Input{Kind: "char", Value: &v}
			}
		}
	}
	a.ID, err = Identity(a.Corpus)
	if err != nil {
		t.Fatal(err)
	}
	original, _ := json.Marshal(a)
	n, err := Check(a, func(seed json.RawMessage) (Component, error) {
		d, e := NewDial(seed)
		for i := range seed {
			seed[i] = ' '
		}
		return mutatingInput{d}, e
	})
	if err != nil || n != 36 {
		t.Fatalf("%d %v", n, err)
	}
	current, _ := json.Marshal(a)
	if !bytes.Equal(original, current) {
		t.Fatal("consumer changed corpus")
	}
}

type reusedView struct {
	Component
	rows []Row
}

func (d *reusedView) View() View {
	view := d.Component.View()
	if d.rows == nil {
		d.rows = view.Rows
	}
	d.rows[0].Value = view.Rows[0].Value
	d.rows[0].Adjustable = false
	view.Rows = d.rows
	return view
}

func TestReusedViewDoesNotRewriteDeparture(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	total := PropertyResult{Name: names[2], Outcome: Outcome{Kind: "not_applicable"}}
	for i := range a.Corpus.Events {
		e := &a.Corpus.Events[i]
		e.Trace.Initial.View.Rows[0].Adjustable = false
		for j := range e.Trace.Steps {
			e.Trace.Steps[j].After.View.Rows[0].Adjustable = false
		}
		e.Checks = []CheckResult{}
		if total.Outcome.Kind == "violated" {
			continue
		}
		before, after, key := e.Trace.Initial.View, e.Trace.Initial.View, Input{Kind: "other"}
		for _, step := range e.Trace.Steps {
			before, after, key = after, step.After.View, step.Key
		}
		o, err := judge(names[2], e.Kind, before, key, after, Flow{})
		if err != nil {
			t.Fatal(err)
		}
		e.Checks = []CheckResult{{Property: 0, Outcome: o}}
		total.Observations++
		if o.Kind != "not_applicable" {
			total.Applicable++
			total.Outcome = o
			if o.Kind == "held" {
				total.Held++
			}
		}
	}
	if total.Outcome.Kind != "violated" {
		t.Fatal("negative reference has no violation")
	}
	a.Corpus.Properties = []PropertyResult{total}
	a.Corpus.Exploration.Verdict = "violated"
	a.ID, err = Identity(a.Corpus)
	if err != nil {
		t.Fatal(err)
	}
	_, err = Check(a, func(seed json.RawMessage) (Component, error) {
		d, e := NewDial(seed)
		return &reusedView{Component: d}, e
	})
	if err == nil || !strings.Contains(err.Error(), "contains violated properties") {
		t.Fatal(fmt.Sprintf("departure view was retroactively changed: %v", err))
	}
}

type titleView struct {
	Component
	title string
}

func (d titleView) View() View { view := d.Component.View(); view.Title = d.title; return view }

func TestInvalidGoStringsCannotEqualValidReplacementCharacters(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	for i := range a.Corpus.Events {
		e := &a.Corpus.Events[i]
		e.Trace.Initial.View.Title = "\ufffd"
		for j := range e.Trace.Steps {
			e.Trace.Steps[j].After.View.Title = "\ufffd"
		}
	}
	a.ID, err = Identity(a.Corpus)
	if err != nil {
		t.Fatal(err)
	}
	for _, title := range []string{"\ufffd", string([]byte{0xff})} {
		n, err := Check(a, func(seed json.RawMessage) (Component, error) { d, e := NewDial(seed); return titleView{d, title}, e })
		if title == "\ufffd" {
			if err != nil || n != 36 {
				t.Fatalf("valid replacement rejected: %d %v", n, err)
			}
		} else if err == nil {
			t.Fatal("invalid UTF-8 normalized to matching evidence")
		}
	}
}

type sharedFlow struct {
	Component
	applied *bool
}

func (d *sharedFlow) Handle(key Input) Flow {
	flow := d.Component.Handle(key)
	d.applied = flow.Applied
	return flow
}
func (d *sharedFlow) View() View {
	if d.applied != nil {
		*d.applied = true
	}
	return d.Component.View()
}

func TestViewCannotRewriteTheReturnedFlow(t *testing.T) {
	a, err := ReadArtifact(fixture(t))
	if err != nil {
		t.Fatal(err)
	}
	n, err := Check(a, func(seed json.RawMessage) (Component, error) {
		d, e := NewDial(seed)
		return &sharedFlow{Component: d}, e
	})
	if err != nil || n != 36 {
		t.Fatalf("flow changed after Handle returned: %d %v", n, err)
	}
}

func TestAnEmptyViolationDetailRemainsPresent(t *testing.T) {
	empty := ""
	actual, err := Canonical(Outcome{Kind: "violated", Detail: &empty})
	if err != nil {
		t.Fatal(err)
	}
	expected, err := canonicalRaw([]byte(`{"kind":"violated","detail":""}`))
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(actual, expected) {
		t.Fatal("typed encoding dropped the required empty violation detail")
	}
}
