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

use crate::{Component, Fingerprint, Flow, Key, Observation, Property, PropertyOutcome, View};

/// What one property actually got to judge.
///
/// The field this replaces was `properties_checked: usize`, set to
/// `properties.len()` before a single observation was looked at — so it meant
/// SUPPLIED while it was named CHECKED, and `is_clean()` treated nonzero as
/// enough. A walk over an alphabet that never reaches a property's domain came
/// back `Clean` with that property never once applied: explore a component over
/// `[Key::Right]` with a claim about Escape and you get transitions, one
/// "checked" property, no violations, `exhausted: true`, therefore `Clean` —
/// without Escape ever being pressed.
///
/// So the report counts what happened instead of what was handed in.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PropertyCoverage {
    /// The property's name, as it would appear on a violation.
    pub name: String,
    /// How many observations it was shown. It stops being consulted once it has
    /// been reported, under the once-per-property policy.
    pub observations: usize,
    /// How many of those were in its DOMAIN — the number that decides whether
    /// its silence is worth anything.
    pub applicable: usize,
    /// How many of the applicable ones it held for.
    pub held: usize,
}

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
    /// What each property actually got to judge — see [`PropertyCoverage`].
    ///
    /// In the report because a report that cannot say this cannot tell
    /// "17,280 states against three properties" from "17,280 states against
    /// nothing", and the second is a walk that judged nothing at all. Per
    /// property and not a count, because "three properties were supplied" and
    /// "three properties were applicable" are different claims and only the
    /// second is evidence.
    pub properties: Vec<PropertyCoverage>,
    /// Each broken property, reported once, with the shortest path to it.
    ///
    /// PRIVATE on purpose, and this is the whole of bug 3c: while it was
    /// public, `assert!(report.violations.is_empty())` was the shortest thing
    /// to write, it is what the README published, and it is exactly the
    /// assertion that cannot refuse a capped run. Read them through
    /// [`Report::verdict`], where the completeness half is not optional.
    violations: Vec<Violation>,
    /// Every DISTINCT view judged, in discovery order — the corpus half of the
    /// harness: a recorded set of states a REIMPLEMENTATION can be held to, in
    /// another language or another framework, without sharing a line of code
    /// with this one.
    ///
    /// Including the view of a state that CLOSED. That is a state an operator
    /// sees, it is where "escape left the draft alone" lives, and a component
    /// whose closing view differs from its open one — every settings panel —
    /// has more of them than it has open states.
    ///
    /// [`Self::exhausted`] is what says whether this is all of them. It is the
    /// SAME flag the violations are judged by, deliberately: this used to be a
    /// second walk with a second flag, and the two disagreed about what "the
    /// states" are — a corpus that is quietly a SUBSET, handed to a
    /// reimplementation as the states it must satisfy, would pass something
    /// that does less.
    pub views: Vec<View>,
    /// FALSE when the search hit a limit; true when it ran out of new states.
    /// A capped search is a SAMPLE, and saying so is the difference between
    /// "nothing is wrong" and "nothing is wrong in the part I looked at".
    ///
    /// (This doc had the polarity backwards, in the one place the crate is
    /// supposed to be honest, and it renders into `cargo doc`. The sentence
    /// above is `docs/PLAN.md:81`'s, which had it right.)
    pub exhausted: bool,
}

/// What a report AMOUNTS TO — the three answers a walk can give.
///
/// A type rather than a pair of booleans because the weak assertion has to be
/// unwriteable, not merely discouraged. `violations.is_empty()` is one
/// conjunct of two, it is the conjunct a truncated run satisfies for free, and
/// while it was reachable it is what the crate's own README published as the
/// idiom to copy. Here the third answer exists, so a consumer who wants the
/// violations has to say what they mean by clean.
///
/// (`stateright`'s `assert_no_discovery` folds the same completeness conjunct
/// inside the helper for the same reason. Two designs, one mechanism.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict<'a> {
    /// The search finished, it checked something, and everything held.
    Clean,
    /// A property did not hold, in a search that finished.
    Violated(&'a [Violation]),
    /// The run cannot clear the component, and says why. Any violations it
    /// did find are real; its SILENCE is what carries no weight.
    Incomplete {
        /// Why this walk proves nothing about what it did not report.
        reason: &'static str,
        /// What it found before it stopped — findings, not a clean bill.
        violations: &'a [Violation],
    },
}

impl Report {
    /// What this walk amounts to.
    ///
    /// `Incomplete` outranks `Violated`: a truncated or vacuous run's findings
    /// are real, but the absence of further findings is not evidence, and the
    /// arm a consumer matches should say the weaker thing.
    #[must_use]
    pub fn verdict(&self) -> Verdict<'_> {
        if let Some(reason) = self.incomplete_because() {
            return Verdict::Incomplete {
                reason,
                violations: &self.violations,
            };
        }
        if self.violations.is_empty() {
            Verdict::Clean
        } else {
            Verdict::Violated(&self.violations)
        }
    }

    /// Why this walk proves nothing, if it does not.
    ///
    /// Four ways a search can come back with no violations and no standing to
    /// say so. The first is a capped search. The next two are the DEGENERATE
    /// runs — nothing supplied, and nothing walked — which are the vacuous
    /// green in its purest form: both of them used to be `is_clean() == true`.
    ///
    /// The last is the one a count could not express, and the reason
    /// `properties_checked` had to become [`PropertyCoverage`]: a property that
    /// was supplied, and walked past, and never once in its domain. Its silence
    /// is not evidence, and a walk that consists only of such silence is not a
    /// clean bill however many states it visited.
    ///
    /// Ordered narrowest-cause-first, so the reason a report gives is the one
    /// that explains the most: an empty alphabet also makes every transition
    /// property inapplicable, and "no key was ever applied" is the finding.
    fn incomplete_because(&self) -> Option<&'static str> {
        if !self.exhausted {
            return Some("STOPPED AT A LIMIT — this is a sample, not a proof");
        }
        if self.properties.is_empty() {
            return Some("NO PROPERTY WAS SUPPLIED — the walk judged nothing");
        }
        if self.transitions == 0 {
            return Some("NO KEY WAS EVER APPLIED — the alphabet was empty");
        }
        if self.properties.iter().any(|p| p.applicable == 0) {
            return Some(
                "A SUPPLIED PROPERTY NEVER APPLIED — its domain was never \
                 reached, so its silence says nothing",
            );
        }
        None
    }

    /// The properties this walk never got to judge — supplied, walked past, and
    /// never once in their domain. Empty is the healthy answer.
    ///
    /// The usual cause is an alphabet that does not contain the key the claim
    /// is about, which `Explorer::new`'s own doc encourages trimming.
    #[must_use]
    pub fn never_applied(&self) -> Vec<&str> {
        self.properties
            .iter()
            .filter(|p| p.applicable == 0)
            .map(|p| p.name.as_str())
            .collect()
    }

    /// Did the search prove what it set out to?
    ///
    /// Every half matters: no violations, AND the search finished, AND it
    /// checked something, AND it walked somewhere.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        matches!(self.verdict(), Verdict::Clean)
    }

    /// The violation of a named property, if it broke.
    ///
    /// A lookup, deliberately — not an emptiness test. "Did THIS claim break"
    /// is a question with an honest answer in any run; "did nothing break" is
    /// the one that needs [`Report::verdict`].
    #[must_use]
    pub fn violation(&self, property: &str) -> Option<&Violation> {
        self.violations.iter().find(|v| v.property == property)
    }
}

impl core::fmt::Display for Report {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        writeln!(
            f,
            "{} states, {} transitions, {} terminal, {} properties{}",
            self.states,
            self.transitions,
            self.terminal_states,
            self.properties.len(),
            match self.incomplete_because() {
                Some(reason) => format!(" ({reason})"),
                None => String::new(),
            }
        )?;
        let never = self.never_applied();
        if !never.is_empty() {
            writeln!(f, "never applicable: {}", never.join(", "))?;
        }
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
        // Starts TRUE and is only ever cleared. Every path that stops early
        // must be able to say so, and a later unconditional assignment would
        // erase an earlier one — see the depth arm below, which is exactly
        // that bug.
        let mut report = Report {
            exhausted: true,
            properties: properties
                .iter()
                .map(|property| PropertyCoverage {
                    name: property.name().to_string(),
                    ..PropertyCoverage::default()
                })
                .collect(),
            ..Report::default()
        };
        let mut seen: HashSet<Fingerprint> = HashSet::new();
        // The shortest path to each state, so a violation can be reproduced.
        let mut queue: VecDeque<Vec<Key>> = VecDeque::new();
        // Reported once per property, keyed on its POSITION in the set rather
        // than on its name: the same broken rule reached by forty paths is one
        // defect, but two DIFFERENT claims that happen to share a name are two,
        // and keying on the name left the second never evaluated at any depth.
        // `Named::new` takes a free-form string with no uniqueness check, and
        // an acceptance set composed from two modules is the point of the crate.
        let mut reported: HashSet<usize> = HashSet::new();

        // Distinct VIEWS, which is not the same set as distinct states: a
        // fingerprint may be finer than the view (folded-in hidden state), and
        // a closing state never enters `seen` at all.
        let mut recorded: HashSet<View> = HashSet::new();

        let start = factory();
        let start_view = start.view();
        seen.insert(start.fingerprint());
        recorded.insert(start_view.clone());
        report.views.push(start_view.clone());
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
                // Truncated here. This used to be silently undone: the
                // assignment at the end of the search overwrote it
                // unconditionally, and since the loop only exits with an EMPTY
                // queue, a depth-capped run reported `exhausted: true` and
                // `is_clean() == true`. A 500-position dial came back as "65
                // states, exhausted" — the crate's headline claim, failing
                // silently, in the one place it is supposed to be honest.
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
                    // left the draft alone" lives — and still a state the
                    // corpus must carry.
                    if recorded.insert(after.clone()) {
                        report.views.push(after.clone());
                    }
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
                    if recorded.insert(after.clone()) {
                        report.views.push(after.clone());
                    }
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
        // Ran out of frontier rather than hitting a limit. `&=` rather than
        // `=`: this narrows the claim, it never restores it, so a depth
        // truncation recorded above survives to the report.
        report.exhausted &= queue.is_empty() && report.states < self.max_states;
        report
    }

    fn check(
        properties: &[&dyn Property],
        observation: &Observation<'_>,
        path: &[Key],
        report: &mut Report,
        reported: &mut HashSet<usize>,
    ) {
        for (index, property) in properties.iter().enumerate() {
            if reported.contains(&index) {
                continue;
            }
            let coverage = &mut report.properties[index];
            coverage.observations += 1;
            match property.check(observation) {
                // Outside its domain: consulted, and it declined to speak. The
                // one outcome that must NOT count towards `applicable`, because
                // that count is the whole difference between a property that
                // held and a property that was never asked anything it knows
                // about.
                PropertyOutcome::NotApplicable => {}
                PropertyOutcome::Held => {
                    coverage.applicable += 1;
                    coverage.held += 1;
                }
                PropertyOutcome::Violated(detail) => {
                    coverage.applicable += 1;
                    reported.insert(index);
                    report.violations.push(Violation {
                        property: property.name().to_string(),
                        path: path.to_vec(),
                        detail,
                    });
                }
            }
        }
    }
}
