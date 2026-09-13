package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	strictjson "github.com/go-json-experiment/json"
	"strconv"
)

type Component interface {
	Handle(Input) Flow
	View() View
	Intent() json.RawMessage
}

func snapshotView(view View) View {
	if view.Rows != nil {
		view.Rows = append([]Row{}, view.Rows...)
	}
	return view
}
func copyInput(key Input) Input {
	if key.Value != nil {
		value := *key.Value
		key.Value = &value
	}
	return key
}

func snapshotFlow(flow Flow) Flow {
	if flow.Applied != nil {
		applied := *flow.Applied
		flow.Applied = &applied
	}
	return flow
}

type Dial struct {
	level, maximum int
	accepted       *int
}

func NewDial(raw json.RawMessage) (Component, error) {
	var s struct {
		Level   uint64 `json:"level"`
		Maximum uint64 `json:"maximum"`
	}
	d := json.NewDecoder(bytes.NewReader(raw))
	d.DisallowUnknownFields()
	if e := d.Decode(&s); e != nil {
		return nil, e
	}
	if !equal(s, raw) || s.Maximum > 20 || s.Level > s.Maximum {
		return nil, fmt.Errorf("invalid dial seed")
	}
	return &Dial{level: int(s.Level), maximum: int(s.Maximum)}, nil
}
func (d *Dial) Handle(k Input) Flow {
	switch k.Kind {
	case "left":
		if d.level > 0 {
			d.level--
		}
	case "right":
		if d.level < d.maximum {
			d.level++
		}
	case "enter":
		level := d.level
		d.accepted = &level
		applied := true
		return Flow{Kind: "close", Applied: &applied}
	case "esc":
		applied := false
		return Flow{Kind: "close", Applied: &applied}
	}
	return Flow{Kind: "stay"}
}
func (d *Dial) View() View {
	return View{Title: "dial", Rows: []Row{{Label: "level", Value: strconv.Itoa(d.level), Note: "", Selected: true, Adjustable: true}}, Footer: "arrows change"}
}
func (d *Dial) Intent() json.RawMessage {
	if d.accepted == nil {
		return json.RawMessage("null")
	}
	b, _ := json.Marshal(map[string]int{"accepted": *d.accepted})
	return b
}
func violation(detail string) Outcome { return Outcome{Kind: "violated", Detail: &detail} }
func judge(name, kind string, before View, key Input, after View, flow Flow) (Outcome, error) {
	outside, held := Outcome{Kind: "not_applicable"}, Outcome{Kind: "held"}
	switch name {
	case names[0]:
		if len(after.Rows) == 0 {
			return outside, nil
		}
		count := 0
		for _, r := range after.Rows {
			if r.Selected {
				count++
			}
		}
		if count == 1 {
			return held, nil
		}
		if count == 0 {
			return violation(fmt.Sprintf("%d rows and nothing selected — a key would act on no row", len(after.Rows))), nil
		}
		return violation(fmt.Sprintf("%d rows claim the cursor at once", count)), nil
	case names[1]:
		if kind != "transition" || key.Kind != "esc" {
			return outside, nil
		}
		if flow.Kind == "close" && flow.Applied != nil && !*flow.Applied {
			return held, nil
		}
		if flow.Kind == "close" {
			return violation("escape closed the component AS AN APPLY"), nil
		}
		return violation("escape left the component open"), nil
	case names[2]:
		if kind != "transition" || key.Kind != "left" && key.Kind != "right" {
			return outside, nil
		}
		for i, old := range before.Rows {
			if old.Selected {
				if i >= len(after.Rows) {
					return outside, nil
				}
				next := after.Rows[i]
				if old.Adjustable || old.Value == next.Value {
					return held, nil
				}
				direction := "Left"
				if key.Kind == "right" {
					direction = "Right"
				}
				return violation(fmt.Sprintf("`%s` is not adjustable but %s changed it from `%s` to `%s`", old.Label, direction, old.Value, next.Value)), nil
			}
		}
		return outside, nil
	default:
		return Outcome{}, fmt.Errorf("unsupported property contract")
	}
}

// Check drives a fresh native consumer for every recorded observation.
func Check(a *Artifact, factory func(json.RawMessage) (Component, error)) (int, error) {
	raw, e := strictjson.Marshal(a)
	if e != nil {
		return 0, e
	}
	a, e = ReadArtifact(raw)
	if e != nil {
		return 0, e
	}
	c := a.Corpus
	if c.Component != componentID {
		return 0, fmt.Errorf("unsupported component contract")
	}
	for index, event := range c.Events {
		d, e := factory(append(json.RawMessage{}, c.Seed...))
		if e != nil {
			return 0, e
		}
		view := snapshotView(d.View())
		if !equal(Snapshot{View: view, Intent: d.Intent()}, event.Trace.Initial) {
			return 0, fmt.Errorf("observation %d: initial snapshot mismatch", index)
		}
		before, key, flow := view, Input{Kind: "other"}, Flow{Kind: "stay"}
		for at, step := range event.Trace.Steps {
			before, key = view, step.Key
			flow = snapshotFlow(d.Handle(copyInput(key)))
			if !equal(flow, step.Flow) {
				return 0, fmt.Errorf("observation %d, key %d: flow mismatch", index, at)
			}
			view = snapshotView(d.View())
			if !equal(view, step.After.View) {
				return 0, fmt.Errorf("observation %d, key %d: view mismatch", index, at)
			}
			if !equal(d.Intent(), step.After.Intent) {
				return 0, fmt.Errorf("observation %d, key %d: intent mismatch", index, at)
			}
		}
		for _, result := range event.Checks {
			actual, e := judge(c.Properties[result.Property].Name, event.Kind, before, key, view, flow)
			if e != nil {
				return 0, e
			}
			if !equal(actual, result.Outcome) {
				return 0, fmt.Errorf("observation %d: property outcome mismatch", index)
			}
		}
	}
	if c.Exploration.Verdict != "clean" {
		return 0, fmt.Errorf("matching evidence is incomplete or contains violated properties")
	}
	return len(c.Events), nil
}
