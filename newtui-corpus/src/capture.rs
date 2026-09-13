use std::cell::RefCell;
use std::rc::Rc;

use newtui::{
    Component, Explorer, Fingerprint, Flow, Key, Observation, Property, PropertyOutcome, Report,
    View,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot<I> {
    pub view: View,
    pub intent: I,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step<I> {
    pub key: Key,
    pub flow: Flow,
    pub after: Snapshot<I>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trace<I> {
    pub initial: Snapshot<I>,
    pub steps: Vec<Step<I>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    State,
    Transition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub property: usize,
    pub outcome: PropertyOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event<I> {
    pub kind: EventKind,
    pub trace: Trace<I>,
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedRun<I> {
    pub report: Report,
    pub events: Vec<Event<I>>,
    pub errors: Vec<String>,
}

/// Observe the existing explorer without changing its search or property policy.
///
/// The intent projection must be pure, like `Component::view`. The wrapper
/// records replay paths in memory; only calls to the final recorder property
/// publish observations. Replay-only transitions never reach that property.
/// The recorder's known positional coverage entry is removed before returning
/// the original report. It is not an acceptance claim.
pub fn capture<C: Component, I: Clone + PartialEq>(
    explorer: &Explorer,
    factory: impl Fn() -> C,
    properties: &[&dyn Property],
    intent: impl Fn(&C) -> I,
) -> CapturedRun<I> {
    let shared = Rc::new(RefCell::new(Sink::new()));
    let wrapped: Vec<_> = properties
        .iter()
        .enumerate()
        .map(|(index, property)| ObservedProperty {
            index,
            property: *property,
            shared: Rc::clone(&shared),
        })
        .collect();
    let recorder = Recorder(Rc::clone(&shared));
    let recorder_index = wrapped.len();
    let mut claims: Vec<&dyn Property> = wrapped.iter().map(|p| p as &dyn Property).collect();
    claims.push(&recorder);
    let mut report = explorer.explore(
        || Traced::new(factory(), &intent, Rc::clone(&shared)),
        &claims,
    );
    report.properties.remove(recorder_index);
    let mut sink = shared.borrow_mut();
    CapturedRun {
        report,
        events: std::mem::take(&mut sink.events),
        errors: std::mem::take(&mut sink.errors),
    }
}

impl<I> Trace<I> {
    fn latest(&self) -> &Snapshot<I> {
        self.steps.last().map_or(&self.initial, |step| &step.after)
    }
}

struct Sink<I> {
    initial: Option<Snapshot<I>>,
    current: Option<Trace<I>>,
    pending: Vec<Check>,
    events: Vec<Event<I>>,
    errors: Vec<String>,
}

impl<I> Sink<I> {
    fn new() -> Self {
        Self {
            initial: None,
            current: None,
            pending: Vec::new(),
            events: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn error(&mut self, reason: &str) {
        if !self.errors.iter().any(|error| error == reason) {
            self.errors.push(reason.into());
        }
    }
}

type Shared<I> = Rc<RefCell<Sink<I>>>;

struct Traced<'a, C, I, F> {
    inner: C,
    intent: &'a F,
    trace: Trace<I>,
    shared: Shared<I>,
}

impl<'a, C: Component, I: Clone + PartialEq, F: Fn(&C) -> I> Traced<'a, C, I, F> {
    fn new(inner: C, intent: &'a F, shared: Shared<I>) -> Self {
        let initial = Snapshot {
            view: inner.view(),
            intent: intent(&inner),
        };
        {
            let mut sink = shared.borrow_mut();
            if let Some(expected) = &sink.initial {
                if *expected != initial {
                    sink.error("the factory produced a different observable initial state");
                }
            } else {
                sink.initial = Some(initial.clone());
            }
        }
        Self {
            inner,
            intent,
            trace: Trace {
                initial,
                steps: Vec::new(),
            },
            shared,
        }
    }
}

impl<C: Component, I: Clone + PartialEq, F: Fn(&C) -> I> Component for Traced<'_, C, I, F> {
    fn handle(&mut self, key: Key) -> Flow {
        let flow = self.inner.handle(key);
        self.trace.steps.push(Step {
            key,
            flow,
            after: Snapshot {
                view: self.inner.view(),
                intent: (self.intent)(&self.inner),
            },
        });
        flow
    }

    fn view(&self) -> View {
        let view = self.inner.view();
        let mut sink = self.shared.borrow_mut();
        if self.trace.latest().view != view {
            sink.error("the component view changed without receiving a key");
        }
        sink.current = Some(self.trace.clone());
        view
    }

    fn fingerprint(&self) -> Fingerprint {
        // Search identity remains entirely inside the core. The sink never
        // sees it, and no portable consumer must reproduce it.
        self.inner.fingerprint()
    }
}

struct ObservedProperty<'a, I> {
    index: usize,
    property: &'a dyn Property,
    shared: Shared<I>,
}

impl<I> Property for ObservedProperty<'_, I> {
    fn name(&self) -> &str {
        self.property.name()
    }

    fn check(&self, observation: &Observation<'_>) -> PropertyOutcome {
        let outcome = self.property.check(observation);
        self.shared.borrow_mut().pending.push(Check {
            property: self.index,
            outcome: outcome.clone(),
        });
        outcome
    }
}

struct Recorder<I>(Shared<I>);

impl<I: Clone> Property for Recorder<I> {
    fn name(&self) -> &str {
        "portable observation recorder (not a property)"
    }

    fn check(&self, observation: &Observation<'_>) -> PropertyOutcome {
        let mut sink = self.0.borrow_mut();
        let Some(trace) = sink.current.clone() else {
            sink.error("the explorer judged an observation without a component snapshot");
            return PropertyOutcome::NotApplicable;
        };
        let (kind, matches) = match observation {
            Observation::State { view } => (EventKind::State, trace.latest().view == **view),
            Observation::Transition {
                from,
                key,
                to,
                flow,
            } => {
                let matches = trace.steps.last().is_some_and(|last| {
                    let prior = trace.steps[..trace.steps.len() - 1]
                        .last()
                        .map_or(&trace.initial, |step| &step.after);
                    last.key == *key
                        && last.flow == *flow
                        && prior.view == **from
                        && last.after.view == **to
                });
                (EventKind::Transition, matches)
            }
        };
        if !matches {
            sink.error("the explorer observation does not match the captured key path");
        }
        if trace
            .steps
            .iter()
            .take(trace.steps.len().saturating_sub(1))
            .any(|step| matches!(step.flow, Flow::Close(_)))
        {
            sink.error("a captured key path continues after the component closed");
        }
        let checks = std::mem::take(&mut sink.pending);
        sink.events.push(Event {
            kind,
            trace,
            checks,
        });
        PropertyOutcome::NotApplicable
    }
}
