//! Generates `spec/tla/lib/RustObs.tla` — the ONLY tie between the TLA+ model and
//! this crate's code.
//!
//! Run: `cargo run --example gen_model > spec/tla/lib/RustObs.tla`
//! Checked by: `scripts/check-model.sh` (regenerate into a tempdir and diff).
//!
//! # What this is, and what it is not
//!
//! `Machine` below implements, in Rust, exactly the transition function
//! `Step`/`Closes` that `spec/tla/ExplorerReplay.tla` hard-codes. This program
//! runs the REAL `Explorer::explore` over it at each depth cap the model
//! sweeps, and emits the four report counters as TLA+ definitions. The model
//! computes those same four numbers ITSELF, from its own transitions, and
//! `ModelMatchesRust` compares them.
//!
//! That direction matters. Regeneration rewrites only the Rust side, so a
//! model that has drifted from `explore.rs` stays RED after regeneration — it
//! cannot heal into "somebody edited the number to match", which is exactly
//! what a golden vector does when the model is a hand-written re-implementation.
//!
//! It is not a proof that the model refines `explore.rs`. It binds five
//! observations per cap in two world modes. What it buys is that a change to
//! the depth arm, to the narrowing at the end of the search, to terminal
//! counting or to the dedupe moves a number here and turns the model red —
//! which is the failure the last one of these shipped through.
//!
//! # Two worlds, because the departure guard exists now
//!
//! `"free"` is the pure factory: every product is a fresh `p0`. `"drain"` is
//! the model's impure world, and it is the one the guard exists for — the
//! FIRST product starts at the warm decoy `w0` and every later one at `p0`,
//! which is what a `OnceLock` whose `take()` drains does to a walk. The model
//! defines exactly that in `InitNode`/`ReplayNode`.
//!
//! Binding "drain" is what stops `GuardOn = TRUE` from being a description of
//! code nobody ran: the numbers below say the crate records ONE divergence,
//! judges nothing past it, and leaves `report.exhausted` alone — and the model
//! has to compute the same five facts from its own transitions.

use newtui::{Component, Explorer, Flow, Key, Row, View};

/// The five-node machine `ExplorerReplay.tla` models, as a real component.
///
/// `p0 -right-> p1 -right-> p2 -right-> p2`, and `esc` closes from anywhere
/// into `bye`. `w0` is the model's unreachable decoy start; it is never
/// entered from `p0` and so never appears in this walk.
struct Machine {
    node: &'static str,
}

impl Component for Machine {
    fn handle(&mut self, key: Key) -> Flow {
        match key {
            Key::Esc => {
                self.node = "bye";
                Flow::Close(false)
            }
            Key::Right => {
                self.node = match self.node {
                    "p0" => "p1",
                    "p1" => "p2",
                    other => other,
                };
                Flow::Stay
            }
            _ => Flow::Stay,
        }
    }

    fn view(&self) -> View {
        // The node is in the view, so the DEFAULT fingerprint (`of_view`) is
        // injective on nodes — which is what lets the model's `FpOf` be the
        // identity honestly rather than by assumption.
        View::titled("machine").row(Row::new("node", self.node))
    }
}

/// The depth caps the model sweeps. Must equal `DepthCaps` in every `.cfg`;
/// `ExplorerReplay.tla` carries `ASSUME DepthCaps \subseteq ObsCaps`.
const CAPS: [usize; 2] = [1, 3];

/// The model's warm decoy start node, `WarmStart` in every `.cfg`.
const WARM: &str = "w0";

/// A factory with the model's `"drain"` semantics: the first product starts
/// warm, every later one starts cold.
///
/// This is not a contrivance for the model's benefit — it is the shape of the
/// bug the guard exists for, and the crate has shipped one: a process-global
/// `OnceLock<Mutex<..>>` that an earlier call has already emptied hands the
/// first component something no later component can get.
struct Draining {
    built: std::cell::Cell<usize>,
}

impl Draining {
    fn build(&self) -> Machine {
        let n = self.built.get();
        self.built.set(n + 1);
        Machine {
            node: if n == 0 { WARM } else { "p0" },
        }
    }
}

fn main() {
    let alphabet = [Key::Right, Key::Esc];
    // `max_states` is deliberately left at its default: the model does not
    // model that limit, so a run that hit it would be comparing two different
    // machines.
    let free: Vec<_> = CAPS
        .iter()
        .map(|&cap| {
            Explorer::new(alphabet)
                .max_depth(cap)
                .explore(|| Machine { node: "p0" }, &[])
        })
        .collect();
    let drain: Vec<_> = CAPS
        .iter()
        .map(|&cap| {
            let factory = Draining {
                built: std::cell::Cell::new(0),
            };
            Explorer::new(alphabet)
                .max_depth(cap)
                .explore(|| factory.build(), &[])
        })
        .collect();
    let worlds: [(&str, &Vec<newtui::Report>); 2] = [("free", &free), ("drain", &drain)];

    let caps = CAPS
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(", ");

    let arm = |reports: &[newtui::Report], f: &dyn Fn(&newtui::Report) -> String| {
        CAPS.iter()
            .enumerate()
            .map(|(i, cap)| format!("d = {cap} -> {}", f(&reports[i])))
            .collect::<Vec<_>>()
            .join(" [] ")
    };
    // One function per observation, keyed by world then by cap, so the model
    // reads `ObsStates[world][cap]` and a world it does not bind is a TLC
    // error rather than a plausible default.
    let by_world = |name: &str, f: &dyn Fn(&newtui::Report) -> String| {
        let arms = worlds
            .iter()
            .map(|(world, reports)| {
                format!(
                    "w = \"{world}\" -> [d \\in ObsCaps |-> CASE {}]",
                    arm(reports, f)
                )
            })
            .collect::<Vec<_>>()
            .join("\n                        [] ");
        println!("{name} == [w \\in ObsWorlds |-> CASE {arms}]");
    };

    // `CASE` with no `OTHER`: an unmatched cap is a TLC error rather than a
    // silently plausible default.
    println!("---------------------------- MODULE RustObs ----------------------------");
    println!("\\* GENERATED by `cargo run --example gen_model` — DO NOT EDIT.");
    println!("\\*");
    println!("\\* Observed report facts from REAL `Explorer::explore` runs over the");
    println!("\\* component in `examples/gen_model.rs`, which implements exactly the");
    println!("\\* `Step`/`Closes` transition function `ExplorerReplay.tla` hard-codes.");
    println!("\\* `ModelMatchesRust` compares these against counters the model computes");
    println!("\\* for itself. Regenerate with `scripts/check-model.sh --write`; that");
    println!("\\* script also fails if the committed copy has drifted.");
    println!("EXTENDS Naturals");
    println!();
    let bool_of = |b: bool| String::from(if b { "TRUE" } else { "FALSE" });
    println!("ObsCaps   == {{{caps}}}");
    println!(
        "ObsWorlds == {{{}}}",
        worlds
            .iter()
            .map(|(w, _)| format!("\"{w}\""))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();
    by_world("ObsStates     ", &|r| r.states.to_string());
    by_world("ObsTransitions", &|r| r.transitions.to_string());
    by_world("ObsTerminal   ", &|r| r.terminal_states.to_string());
    by_world("ObsExhausted  ", &|r| bool_of(r.exhausted));
    by_world("ObsDiverged   ", &|r| bool_of(!r.divergences().is_empty()));
    println!("=======================================================================");
}
