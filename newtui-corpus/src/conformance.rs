use newtui::{Component, Explorer, Property};
use serde_json::Value;

use crate::model::*;

pub fn export<C: Component>(
    component: &str,
    seed: Value,
    domain: Domain,
    factory: impl Fn() -> C,
    properties: &[&dyn Property],
    intent: impl Fn(&C) -> Value,
) -> Result<Artifact, String> {
    domain.validate()?;
    crate::value::validate(&seed)?;
    if properties.len() > 64 {
        return Err("at most 64 properties may be exported".into());
    }
    let explorer = Explorer::new(domain.alphabet.iter().map(newtui::Key::from))
        .max_states(domain.max_states as usize)
        .max_depth(domain.max_depth as usize);
    let run = crate::capture(&explorer, factory, properties, intent);
    if !run.errors.is_empty() {
        return Err(format!("inconsistent capture: {}", run.errors.join("; ")));
    }
    let (verdict, incomplete_reason) = match run.report.verdict() {
        newtui::Verdict::Clean => (Verdict::Clean, None),
        newtui::Verdict::Violated(_) => (Verdict::Violated, None),
        newtui::Verdict::Incomplete { reason, .. } => (Verdict::Incomplete, Some(reason.into())),
    };
    let events = run
        .events
        .into_iter()
        .map(Event::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    let results = run
        .report
        .properties
        .iter()
        .enumerate()
        .map(|(index, p)| {
            let outcome = events
                .iter()
                .flat_map(|event| &event.checks)
                .find(|check| {
                    check.property == index as u64
                        && matches!(check.outcome, Outcome::Violated { .. })
                })
                .map_or_else(
                    || {
                        if p.applicable == 0 {
                            Outcome::NotApplicable
                        } else {
                            Outcome::Held
                        }
                    },
                    |check| check.outcome.clone(),
                );
            PropertyResult {
                name: p.name.clone(),
                observations: p.observations as u64,
                applicable: p.applicable as u64,
                held: p.held as u64,
                outcome,
            }
        })
        .collect();
    let replay_issues = run
        .report
        .divergences()
        .iter()
        .map(|issue| {
            Ok(ReplayIssue {
                path: issue
                    .path
                    .iter()
                    .copied()
                    .map(Input::try_from)
                    .collect::<Result<_, _>>()?,
                reason: match issue.reason {
                    newtui::DivergenceReason::DifferentState => ReplayReason::DifferentState,
                    newtui::DivergenceReason::ClosedDuringReplay { at } => {
                        ReplayReason::ClosedDuringReplay { at: at as u64 }
                    }
                },
            })
        })
        .collect::<Result<_, String>>()?;
    Artifact::mint(Corpus {
        schema: SCHEMA.into(),
        component: component.into(),
        seed,
        domain,
        exploration: Exploration {
            states: run.report.states as u64,
            transitions: run.report.transitions as u64,
            terminal_states: run.report.terminal_states as u64,
            exhausted: run.report.exhausted,
            verdict,
            incomplete_reason,
            replay_issues,
        },
        properties: results,
        events,
    })
}

/// Replay the declared observations and properties against a fresh consumer.
///
/// The caller must first select an implementation for `artifact.corpus.component`;
/// this generic function cannot infer the host's versioned component contract.
/// Matching incomplete or violated evidence is an error, never conformance.
pub fn check<C: Component>(
    artifact: &Artifact,
    factory: impl Fn(&Value) -> Result<C, String>,
    properties: &[&dyn Property],
    intent: impl Fn(&C) -> Value,
) -> Result<usize, String> {
    artifact.validate()?;
    let corpus = &artifact.corpus;
    if properties
        .iter()
        .map(|property| property.name())
        .collect::<Vec<_>>()
        != corpus
            .properties
            .iter()
            .map(|property| property.name.as_str())
            .collect::<Vec<_>>()
    {
        return Err(
            "consumer property positions/names differ from the declared property set".into(),
        );
    }
    for (index, event) in corpus.events.iter().enumerate() {
        let mut component = factory(&corpus.seed)?;
        let mut view = component.view();
        if View::from(view.clone()) != event.trace.initial.view
            || intent(&component) != event.trace.initial.intent
        {
            return Err(format!("observation {index}: initial snapshot mismatch"));
        }
        let mut before = view.clone();
        let mut flow = newtui::Flow::Stay;
        let mut key = newtui::Key::Other;
        for (at, step) in event.trace.steps.iter().enumerate() {
            before = view;
            key = (&step.key).into();
            flow = component.handle(key);
            if Flow::from(flow) != step.flow {
                return Err(format!("observation {index}, key {at}: flow mismatch"));
            }
            view = component.view();
            if View::from(view.clone()) != step.after.view {
                return Err(format!("observation {index}, key {at}: view mismatch"));
            }
            if intent(&component) != step.after.intent {
                return Err(format!("observation {index}, key {at}: intent mismatch"));
            }
        }
        let observation = match event.kind {
            EventKind::State => newtui::Observation::State { view: &view },
            EventKind::Transition => newtui::Observation::Transition {
                from: &before,
                key,
                to: &view,
                flow,
            },
        };
        for check in &event.checks {
            if Outcome::from(properties[check.property as usize].check(&observation))
                != check.outcome
            {
                return Err(format!(
                    "observation {index}: property {} outcome mismatch",
                    check.property
                ));
            }
        }
    }
    match corpus.exploration.verdict {
        Verdict::Clean => {}
        Verdict::Incomplete => {
            return Err(format!(
                "matching evidence is incomplete: {}",
                corpus
                    .exploration
                    .incomplete_reason
                    .as_deref()
                    .unwrap_or("unknown reason")
            ))
        }
        Verdict::Violated => return Err("matching evidence contains violated properties".into()),
    }
    Ok(artifact.corpus.events.len())
}
