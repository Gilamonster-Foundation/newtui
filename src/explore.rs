//! Walking everything a component can reach.
//!
//! # Why states and not sequences
//!
//! Enumerating key SEQUENCES explodes: six keys to depth eight is 1.6 million
//! paths, and almost all of them arrive somewhere already seen. Enumerating
//! STATES does not: a component is a finite machine, and the reachable set is
//! usually dozens. The whole search rests on `Component::fingerprint` telling
//! two states apart — see its docs for what happens when it is too coarse.
//!
//! # Why breadth-first
//!
//! Because the first path to a violation is then the SHORTEST one. A failure
//! reports the minimal key sequence that reaches it, by construction. That is
//! the thing property-based testing spends a shrinking phase to approximate,
//! and here it falls out of the queue order for free.
//!
//! # Why replay instead of clone
//!
//! Each state is reached by replaying its key path into a FRESH component from
//! the factory. That costs more than cloning, and buys two things worth more
//! than the cycles: a component needs no `Clone` bound (several hold injected
//! closures that cannot be cloned), and every state in the report is reachable
//! from a real start — a cloned mid-search state can drift from anything a
//! session could actually produce.

use std::collections::{HashSet, VecDeque};

use crate::{Component, Fingerprint, Flow, Key, Observation, Property, View};

/// A property that did not hold, and the shortest way to reproduce it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Which claim did not hold.
    pub property: String,
    /// The MINIMAL key sequence from the initial state. Breadth-first order is
    /// what makes it minimal.
    pub path: Vec<Key>,
    /// What went wrong, phrased for whoever reads the failure.
    pub detail: String,
}

impl core::fmt::Display for Violation {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let path = if self.path.is_empty() {
            "(the initial state)".to_string()
        } else {
            self.path
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(" ")
        };
        write!(f, "{}\n  after: {path}\n  {}", self.property, self.detail)
    }
}

/// What an exploration found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Distinct states reached.
    pub states: usize,
    /// Key applications made.
    pub transitions: usize,
    /// States that closed the component — a search does not continue past one,
    /// because a closed component is not there to receive another key.
    pub terminal_states: usize,
    /// Each broken property, reported once, with the shortest path to it.
    pub violations: Vec<Violation>,
    /// True when the search stopped at a limit rather than because it ran out
    /// of new states. A capped search is a SAMPLE, and saying so is the
    /// difference between "nothing is wrong" and "nothing is wrong in the part
    /// I looked at".
    pub exhausted: bool,
}

impl Report {
    /// Did the search prove what it set out to?
    ///
    /// Both halves matter: no violations, AND the search actually finished. A
    /// capped run with no violations has not cleared the component.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.violations.is_empty() && self.exhausted
    }
}

impl core::fmt::Display for Report {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "{} states, {} transitions, {} terminal{}",
            self.states,
            self.transitions,
            self.terminal_states,
            if self.exhausted {
                ""
            } else {
                " (STOPPED AT A LIMIT — this is a sample, not a proof)"
            }
        )?;
        if self.violations.is_empty() {
            return write!(f, "no violations");
        }
        writeln!(f, "{} violations:", self.violations.len())?;
        for violation in &self.violations {
            writeln!(f, "\n{violation}")?;
        }
        Ok(())
    }
}

/// Walks every reachable state of a component, checking properties.
pub struct Explorer {
    alphabet: Vec<Key>,
    max_states: usize,
    max_depth: usize,
}

impl Explorer {
    /// The keys this component can receive.
    ///
    /// The alphabet is the caller's, because it is part of what a component
    /// claims: a settings panel that only answers arrows, Enter and Esc should
    /// be explored over those, and one with a text field passes the characters
    /// it accepts. A key nothing is defined for costs a transition per state
    /// and finds nothing.
    #[must_use]
    pub fn new(alphabet: impl IntoIterator<Item = Key>) -> Self {
        Self {
            alphabet: alphabet.into_iter().collect(),
            // Generous, and a backstop rather than a target: a component whose
            // reachable set runs to five figures has state the fingerprint is
            // splitting too finely, and the report says the search was capped
            // rather than pretending it finished.
            max_states: 50_000,
            max_depth: 64,
        }
    }

    /// Stop after this many distinct states.
    #[must_use]
    pub fn max_states(mut self, max: usize) -> Self {
        self.max_states = max;
        self
    }

    /// Stop at this key depth.
    #[must_use]
    pub fn max_depth(mut self, max: usize) -> Self {
        self.max_depth = max;
        self
    }

    /// Walk the component, checking every property at every state and
    /// transition.
    ///
    /// `factory` builds a FRESH component — this is where mock data sources
    /// come in. Everything the component reads must arrive through it, or the
    /// search is exploring one thing and the properties are judging another.
    pub fn explore<C: Component>(
        &self,
        factory: impl Fn() -> C,
        properties: &[&dyn Property],
    ) -> Report {
        let mut report = Report::default();
        let mut seen: HashSet<Fingerprint> = HashSet::new();
        // The shortest path to each state, so a violation can be reproduced.
        let mut queue: VecDeque<Vec<Key>> = VecDeque::new();
        // Reported once per property: the same broken rule reached by forty
        // paths is one defect, and forty copies of it buries the other three.
        let mut reported: HashSet<String> = HashSet::new();

        let start = factory();
        let start_view = start.view();
        seen.insert(start.fingerprint());
        report.states = 1;
        Self::check(
            properties,
            &Observation::State { view: &start_view },
            &[],
            &mut report,
            &mut reported,
        );
        queue.push_back(Vec::new());

        while let Some(path) = queue.pop_front() {
            if path.len() >= self.max_depth {
                report.exhausted = false;
                continue;
            }
            for key in &self.alphabet {
                if report.states >= self.max_states {
                    return Report {
                        exhausted: false,
                        ..report
                    };
                }
                // Replay to this state, then take one more step.
                let mut component = factory();
                for replayed in &path {
                    component.handle(*replayed);
                }
                let before = component.view();
                let flow = component.handle(*key);
                let after = component.view();
                report.transitions += 1;

                let mut next_path = path.clone();
                next_path.push(*key);
                Self::check(
                    properties,
                    &Observation::Transition {
                        from: &before,
                        key: *key,
                        to: &after,
                        flow,
                    },
                    &next_path,
                    &mut report,
                    &mut reported,
                );

                if matches!(flow, Flow::Close(_)) {
                    // A closed component receives no more keys. Its final view
                    // is still a state worth judging — that is where "escape
                    // left the draft alone" lives.
                    report.terminal_states += 1;
                    Self::check(
                        properties,
                        &Observation::State { view: &after },
                        &next_path,
                        &mut report,
                        &mut reported,
                    );
                    continue;
                }

                if seen.insert(component.fingerprint()) {
                    report.states += 1;
                    Self::check(
                        properties,
                        &Observation::State { view: &after },
                        &next_path,
                        &mut report,
                        &mut reported,
                    );
                    queue.push_back(next_path);
                }
            }
        }
        // Ran out of frontier rather than hitting a limit: the search saw
        // everything reachable.
        report.exhausted = queue.is_empty() && report.states < self.max_states;
        report
    }

    fn check(
        properties: &[&dyn Property],
        observation: &Observation<'_>,
        path: &[Key],
        report: &mut Report,
        reported: &mut HashSet<String>,
    ) {
        for property in properties {
            if reported.contains(property.name()) {
                continue;
            }
            if let Err(detail) = property.check(observation) {
                reported.insert(property.name().to_string());
                report.violations.push(Violation {
                    property: property.name().to_string(),
                    path: path.to_vec(),
                    detail,
                });
            }
        }
    }

    /// Every distinct view a component can reach, in discovery order.
    ///
    /// The corpus half of the harness: a recorded set of states is data a
    /// REIMPLEMENTATION can be held to, in another language or another
    /// framework, without sharing a line of code with this one.
    pub fn states<C: Component>(&self, factory: impl Fn() -> C) -> Vec<View> {
        let mut seen: HashSet<Fingerprint> = HashSet::new();
        let mut views = Vec::new();
        let mut queue: VecDeque<Vec<Key>> = VecDeque::new();

        let start = factory();
        seen.insert(start.fingerprint());
        views.push(start.view());
        queue.push_back(Vec::new());

        while let Some(path) = queue.pop_front() {
            if path.len() >= self.max_depth || views.len() >= self.max_states {
                break;
            }
            for key in &self.alphabet {
                let mut component = factory();
                for replayed in &path {
                    component.handle(*replayed);
                }
                if matches!(component.handle(*key), Flow::Close(_)) {
                    continue;
                }
                if seen.insert(component.fingerprint()) {
                    views.push(component.view());
                    let mut next = path.clone();
                    next.push(*key);
                    queue.push_back(next);
                }
            }
        }
        views
    }
}
