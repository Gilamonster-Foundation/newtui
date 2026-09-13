package main

import (
	"encoding/json"
	"fmt"
	"unicode/utf8"
)

func keyID(k Input) (string, error) {
	switch k.Kind {
	case "up", "down", "left", "right", "enter", "esc", "backspace", "tab", "back_tab", "home", "end", "page_up", "page_down", "other":
		if k.Value != nil {
			return "", fmt.Errorf("unexpected key value")
		}
	case "char", "ctrl":
		if k.Value == nil || !utf8.ValidString(*k.Value) || utf8.RuneCountInString(*k.Value) != 1 {
			return "", fmt.Errorf("character key needs one Unicode scalar")
		}
	default:
		return "", fmt.Errorf("unknown key")
	}
	b, _ := json.Marshal(k)
	return string(b), nil
}
func validFlow(f Flow) bool {
	return f.Kind == "stay" && f.Applied == nil || f.Kind == "close" && f.Applied != nil
}
func validOutcome(o Outcome) bool {
	return (o.Kind == "held" || o.Kind == "not_applicable") && o.Detail == nil || o.Kind == "violated" && o.Detail != nil
}
func pathID(p []Input) string { b, _ := json.Marshal(p); return string(b) }
func pathOf(t Trace) []Input {
	p := make([]Input, len(t.Steps))
	for i, s := range t.Steps {
		p[i] = s.Key
	}
	return p
}

func validate(c Corpus) error {
	bad := func() error { return fmt.Errorf("malformed corpus domain, trace or outcomes") }
	if c.Schema != schema || len(c.Component) == 0 || len(c.Component) > 128 {
		return bad()
	}
	if err := rawPortable(c.Seed); err != nil {
		return err
	}
	d := c.Domain
	if d.MaxStates > 50000 || d.MaxDepth > 64 || d.Alphabet == nil || len(d.Alphabet) > 64 || c.Properties == nil || len(c.Properties) > 64 || len(c.Events) == 0 || len(c.Events) > 100000 || c.Exploration.ReplayIssues == nil {
		return bad()
	}
	alphabet := map[string]bool{}
	for _, k := range d.Alphabet {
		id, e := keyID(k)
		if e != nil || alphabet[id] {
			return bad()
		}
		alphabet[id] = true
	}
	totals := make([]PropertyResult, len(c.Properties))
	for i, p := range c.Properties {
		if !validOutcome(p.Outcome) {
			return bad()
		}
		totals[i] = PropertyResult{Name: p.Name, Outcome: Outcome{Kind: "not_applicable"}}
	}
	opened := map[string]Trace{}
	transitions := map[string]bool{}
	terminal := uint64(0)
	initial := c.Events[0].Trace.Initial
	for index, event := range c.Events {
		t := event.Trace
		if (event.Kind != "state" && event.Kind != "transition") || event.Checks == nil || t.Steps == nil || uint64(len(t.Steps)) > d.MaxDepth || t.Initial.View.Rows == nil || rawPortable(t.Initial.Intent) != nil || !equal(t.Initial, initial) {
			return bad()
		}
		if index == 0 {
			if event.Kind != "state" || len(t.Steps) != 0 {
				return bad()
			}
			opened[pathID([]Input{})] = t
		}
		for at, s := range t.Steps {
			id, e := keyID(s.Key)
			if e != nil || !alphabet[id] || !validFlow(s.Flow) || at+1 < len(t.Steps) && s.Flow.Kind == "close" || s.After.View.Rows == nil || rawPortable(s.After.Intent) != nil {
				return bad()
			}
		}
		path := pathOf(t)
		id := pathID(path)
		if index > 0 && event.Kind == "transition" {
			if uint64(len(opened)) >= d.MaxStates {
				return fmt.Errorf("transition recorded after the state cap")
			}
			if len(path) == 0 || transitions[id] {
				return bad()
			}
			parent, ok := opened[pathID(path[:len(path)-1])]
			if !ok || !equal(t.Steps[:len(t.Steps)-1], parent.Steps) {
				return bad()
			}
			transitions[id] = true
			if t.Steps[len(t.Steps)-1].Flow.Kind == "close" && (index+1 == len(c.Events) || c.Events[index+1].Kind != "state" || !equal(c.Events[index+1].Trace, t)) {
				return bad()
			}
		} else if index > 0 {
			if c.Events[index-1].Kind != "transition" || !equal(c.Events[index-1].Trace, t) {
				return bad()
			}
			if t.Steps[len(t.Steps)-1].Flow.Kind == "stay" {
				if _, ok := opened[id]; ok {
					return bad()
				}
				opened[id] = t
			} else {
				terminal++
			}
		}
		at := 0
		for i := range totals {
			if totals[i].Outcome.Kind == "violated" {
				continue
			}
			if at >= len(event.Checks) || event.Checks[at].Property != uint64(i) || !validOutcome(event.Checks[at].Outcome) {
				return bad()
			}
			o := event.Checks[at].Outcome
			at++
			totals[i].Observations++
			if o.Kind != "not_applicable" {
				totals[i].Applicable++
				totals[i].Outcome = o
				if o.Kind == "held" {
					totals[i].Held++
				}
			}
		}
		if at != len(event.Checks) {
			return bad()
		}
	}
	r := c.Exploration
	if !equal(totals, c.Properties) || r.States != uint64(len(opened)) || r.Transitions != uint64(len(transitions)) || r.TerminalStates != terminal {
		return bad()
	}
	capped := uint64(len(opened)) >= d.MaxStates
	for _, t := range opened {
		capped = capped || uint64(len(t.Steps)) >= d.MaxDepth
	}
	if r.Exhausted == capped {
		return bad()
	}
	for _, issue := range r.ReplayIssues {
		if issue.Path == nil {
			return bad()
		}
		if _, ok := opened[pathID(issue.Path)]; !ok {
			return bad()
		}
		if issue.Reason.Kind == "different_state" {
			if issue.Reason.At != nil {
				return bad()
			}
		} else if issue.Reason.Kind != "closed_during_replay" || issue.Reason.At == nil || *issue.Reason.At >= uint64(len(issue.Path)) {
			return bad()
		}
	}
	if r.Exhausted && len(r.ReplayIssues) == 0 {
		for _, t := range opened {
			for _, k := range d.Alphabet {
				p := append(pathOf(t), k)
				if !transitions[pathID(p)] {
					return bad()
				}
			}
		}
	}
	incomplete := !r.Exhausted || len(r.ReplayIssues) > 0 || len(totals) == 0 || len(transitions) == 0
	violated := false
	for _, p := range totals {
		incomplete = incomplete || p.Applicable == 0
		violated = violated || p.Outcome.Kind == "violated"
	}
	verdict := "clean"
	if incomplete {
		verdict = "incomplete"
	} else if violated {
		verdict = "violated"
	}
	if r.Verdict != verdict || incomplete && (r.IncompleteReason == nil || *r.IncompleteReason == "") || !incomplete && r.IncompleteReason != nil {
		return bad()
	}
	return nil
}
