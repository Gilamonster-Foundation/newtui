use std::collections::{BTreeMap, BTreeSet};

use crate::model::*;

impl Domain {
    pub fn validate(&self) -> Result<(), String> {
        if self.max_depth > 64 || self.max_states > 50_000 || self.alphabet.len() > 64 {
            return Err("domain exceeds 64 keys, depth 64, or 50000 states".into());
        }
        if self.alphabet.iter().collect::<BTreeSet<_>>().len() != self.alphabet.len() {
            return Err("the exploration alphabet contains a duplicate key".into());
        }
        Ok(())
    }
}

impl Corpus {
    /// Check application semantics separately from content integrity. A valid
    /// CID can identify a malformed corpus; it cannot make that corpus true.
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA {
            return Err("unsupported corpus schema".into());
        }
        if self.component.is_empty() || self.component.len() > 128 {
            return Err("a component contract name must contain 1 to 128 UTF-8 bytes".into());
        }
        self.domain.validate()?;
        crate::value::validate(&self.seed)?;
        if self.properties.len() > 64 || self.events.len() > 100_000 {
            return Err("corpus exceeds 64 properties or 100000 observations".into());
        }
        let Some(first) = self.events.first() else {
            return Err("the initial state is missing".into());
        };
        if first.kind != EventKind::State || !first.trace.steps.is_empty() {
            return Err("the first observation must be the fresh seed state".into());
        }
        let mut open = BTreeMap::from([(Vec::new(), &first.trace)]);
        let mut transitions = BTreeSet::new();
        let mut terminal = 0;
        let mut totals: Vec<PropertyResult> = self
            .properties
            .iter()
            .map(|p| PropertyResult {
                name: p.name.clone(),
                observations: 0,
                applicable: 0,
                held: 0,
                outcome: Outcome::NotApplicable,
            })
            .collect();
        for (index, event) in self.events.iter().enumerate() {
            let trace = &event.trace;
            if trace.initial != first.trace.initial
                || trace.steps.len() as u64 > self.domain.max_depth
            {
                return Err("a trace has a different seed or exceeds the declared depth".into());
            }
            crate::value::validate(&trace.initial.intent)?;
            let path: Vec<_> = trace.steps.iter().map(|step| step.key.clone()).collect();
            for (at, step) in trace.steps.iter().enumerate() {
                crate::value::validate(&step.after.intent)?;
                if !self.domain.alphabet.contains(&step.key) {
                    return Err("a trace uses a key outside its declared alphabet".into());
                }
                if at + 1 < trace.steps.len() && step.flow != Flow::Stay {
                    return Err("a trace continues after the component closed".into());
                }
            }
            if index > 0 {
                match event.kind {
                    EventKind::Transition => {
                        if open.len() as u64 >= self.domain.max_states {
                            return Err("a transition was recorded after the state cap".into());
                        }
                        if path.is_empty() || !transitions.insert(path.clone()) {
                            return Err("an empty or duplicate transition path was recorded".into());
                        }
                        let Some(prior) = open.get(&path[..path.len() - 1]) else {
                            return Err(
                                "a transition does not depart from a discovered open state".into(),
                            );
                        };
                        if trace.steps[..trace.steps.len() - 1] != prior.steps {
                            return Err(
                                "the replay prefix differs from its discovered trace".into()
                            );
                        }
                        if trace.steps.last().unwrap().flow != Flow::Stay
                            && !self.events.get(index + 1).is_some_and(|next| {
                                next.kind == EventKind::State && next.trace == *trace
                            })
                        {
                            return Err(
                                "a closing transition is missing its terminal state observation"
                                    .into(),
                            );
                        }
                    }
                    EventKind::State => {
                        let previous = &self.events[index - 1];
                        if previous.kind != EventKind::Transition || previous.trace != *trace {
                            return Err("a state observation does not follow its transition".into());
                        }
                        if trace.steps.last().unwrap().flow == Flow::Stay {
                            if open.insert(path, trace).is_some() {
                                return Err("an open state path was discovered twice".into());
                            }
                        } else {
                            terminal += 1;
                        }
                    }
                }
            }
            let applicable_indices: Vec<_> = totals
                .iter()
                .enumerate()
                .filter(|(_, p)| !matches!(p.outcome, Outcome::Violated { .. }))
                .map(|(i, _)| i as u64)
                .collect();
            if event
                .checks
                .iter()
                .map(|check| check.property)
                .collect::<Vec<_>>()
                != applicable_indices
            {
                return Err(
                    "property checks lost positional identity, order, or retirement".into(),
                );
            }
            for check in &event.checks {
                let total = &mut totals[check.property as usize];
                total.observations += 1;
                match &check.outcome {
                    Outcome::NotApplicable => {}
                    Outcome::Held => {
                        total.applicable += 1;
                        total.held += 1;
                        total.outcome = Outcome::Held;
                    }
                    Outcome::Violated { .. } => {
                        total.applicable += 1;
                        total.outcome = check.outcome.clone();
                    }
                }
            }
        }
        let report = &self.exploration;
        if totals != self.properties
            || open.len() as u64 != report.states
            || transitions.len() as u64 != report.transitions
            || terminal != report.terminal_states
        {
            return Err(
                "reported counts or property outcomes disagree with the observations".into(),
            );
        }
        let depth_capped = open
            .keys()
            .any(|path| path.len() as u64 >= self.domain.max_depth);
        let state_capped = report.states >= self.domain.max_states;
        if report.exhausted == (depth_capped || state_capped) {
            return Err("exhaustion disagrees with the declared search bounds".into());
        }
        for issue in &report.replay_issues {
            if !open.contains_key(&issue.path) {
                return Err("a replay issue names an undiscovered departure".into());
            }
            if let ReplayReason::ClosedDuringReplay { at } = issue.reason {
                if at >= issue.path.len() as u64 {
                    return Err("a replay-close index is outside its path".into());
                }
            }
        }
        if report.exhausted && report.replay_issues.is_empty() {
            for path in open.keys() {
                for key in &self.domain.alphabet {
                    let mut edge = path.clone();
                    edge.push(key.clone());
                    if !transitions.contains(&edge) {
                        return Err("an exhausted walk is missing an outgoing transition".into());
                    }
                }
            }
        }
        let incomplete = !report.exhausted
            || !report.replay_issues.is_empty()
            || self.properties.is_empty()
            || report.transitions == 0
            || self.properties.iter().any(|p| p.applicable == 0);
        let expected = if incomplete {
            Verdict::Incomplete
        } else if self
            .properties
            .iter()
            .any(|p| matches!(p.outcome, Outcome::Violated { .. }))
        {
            Verdict::Violated
        } else {
            Verdict::Clean
        };
        if report.verdict != expected
            || report
                .incomplete_reason
                .as_ref()
                .is_some_and(|reason| reason.is_empty())
            || report.incomplete_reason.is_some() != incomplete
        {
            return Err(
                "the verdict or its incomplete explanation contradicts the evidence".into(),
            );
        }
        Ok(())
    }
}
