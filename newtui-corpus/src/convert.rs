use crate::model::*;

impl TryFrom<newtui::Key> for Input {
    type Error = String;
    fn try_from(key: newtui::Key) -> Result<Self, String> {
        use newtui::Key as K;
        Ok(match key {
            K::Up => Self::Up,
            K::Down => Self::Down,
            K::Left => Self::Left,
            K::Right => Self::Right,
            K::Enter => Self::Enter,
            K::Esc => Self::Esc,
            K::Backspace => Self::Backspace,
            K::Tab => Self::Tab,
            K::BackTab => Self::BackTab,
            K::Home => Self::Home,
            K::End => Self::End,
            K::PageUp => Self::PageUp,
            K::PageDown => Self::PageDown,
            K::Char(c) => Self::Char(c),
            K::Ctrl(c) => Self::Ctrl(c),
            K::Other => Self::Other,
            _ => return Err("this key has no representation in corpus schema v1".into()),
        })
    }
}

impl From<&Input> for newtui::Key {
    fn from(key: &Input) -> Self {
        match key {
            Input::Up => Self::Up,
            Input::Down => Self::Down,
            Input::Left => Self::Left,
            Input::Right => Self::Right,
            Input::Enter => Self::Enter,
            Input::Esc => Self::Esc,
            Input::Backspace => Self::Backspace,
            Input::Tab => Self::Tab,
            Input::BackTab => Self::BackTab,
            Input::Home => Self::Home,
            Input::End => Self::End,
            Input::PageUp => Self::PageUp,
            Input::PageDown => Self::PageDown,
            Input::Char(c) => Self::Char(*c),
            Input::Ctrl(c) => Self::Ctrl(*c),
            Input::Other => Self::Other,
        }
    }
}

impl From<newtui::Flow> for Flow {
    fn from(flow: newtui::Flow) -> Self {
        match flow {
            newtui::Flow::Stay => Self::Stay,
            newtui::Flow::Close(applied) => Self::Close { applied },
        }
    }
}

impl From<newtui::PropertyOutcome> for Outcome {
    fn from(outcome: newtui::PropertyOutcome) -> Self {
        match outcome {
            newtui::PropertyOutcome::NotApplicable => Self::NotApplicable,
            newtui::PropertyOutcome::Held => Self::Held,
            newtui::PropertyOutcome::Violated(detail) => Self::Violated { detail },
        }
    }
}

impl From<newtui::View> for View {
    fn from(view: newtui::View) -> Self {
        Self {
            title: view.title,
            footer: view.footer,
            rows: view
                .rows
                .into_iter()
                .map(|row| Row {
                    label: row.label,
                    value: row.value,
                    note: row.note,
                    selected: row.selected,
                    adjustable: row.adjustable,
                })
                .collect(),
        }
    }
}

impl From<crate::Snapshot<serde_json::Value>> for Snapshot {
    fn from(snapshot: crate::Snapshot<serde_json::Value>) -> Self {
        Self {
            view: snapshot.view.into(),
            intent: snapshot.intent,
        }
    }
}

impl TryFrom<crate::Event<serde_json::Value>> for Event {
    type Error = String;
    fn try_from(event: crate::Event<serde_json::Value>) -> Result<Self, String> {
        Ok(Self {
            kind: match event.kind {
                crate::EventKind::State => EventKind::State,
                crate::EventKind::Transition => EventKind::Transition,
            },
            trace: Trace {
                initial: event.trace.initial.into(),
                steps: event
                    .trace
                    .steps
                    .into_iter()
                    .map(|step| {
                        Ok(Step {
                            key: step.key.try_into()?,
                            flow: step.flow.into(),
                            after: step.after.into(),
                        })
                    })
                    .collect::<Result<_, String>>()?,
            },
            checks: event
                .checks
                .into_iter()
                .map(|check| Check {
                    property: check.property as u64,
                    outcome: check.outcome.into(),
                })
                .collect(),
        })
    }
}
