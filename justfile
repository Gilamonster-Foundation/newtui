# justfile for newtui.
#
# `just check` is the full local gate and mirrors .github/workflows/ci.yml.
# Keep the three in step: this file, that workflow, and .githooks/pre-push.
#
# The FORMAL layer (spec/tla/, formal/) mirrors .github/workflows/formal.yml.
# It splits on toolchain cost, and the split is written down in all three
# places — here, the hook header, and that workflow's header:
#
#   `no-sorry` and `model` are in `check` and in the push hook. Both are
#   cheap — a grep and a cargo run — and there is no excuse for a
#   milliseconds-long gate being CI-only.
#
#   `lean`, `lean-mutations`, `tla` and `mutations` are NOT in `check`. They
#   need a Lean toolchain and a pinned 10 MB tla2tools.jar, and requiring both
#   in every developer's pre-push would be disproportionate. They are CI-only
#   BY DESIGN, the way newt-agent's formal.yml records `HOOK PARITY:
#   intentionally NONE`. Run them locally with `just formal`.

# Format, lint, test and document — the whole gate.
check: fmt clippy test doc leaf rust-mutations no-sorry model

# Verify formatting (does not modify files).
fmt:
    cargo fmt -- --check

# Lint at every feature setting that ships. `--no-default-features` is the
# configuration a headless consumer actually gets, so it is linted too.
clippy:
    cargo clippy --all-targets --all-features -- -D warnings
    cargo clippy --all-targets --no-default-features -- -D warnings

# Test in both configurations. Plain `cargo test` includes doctests, which
# `--all-targets` skips — and the README's examples are doctests.
test:
    cargo test --all-features
    cargo test --no-default-features

# Docs must build clean: a broken intra-doc link in a crate whose whole job is
# explaining a seam is a defect in the product, not in the docs.
doc:
    RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features

# The leaf invariant — the runtime closure stays empty. See tests/leaf.rs.
leaf:
    cargo test --test leaf

# Every Rust guard has a defect it provably catches. Named target because
# tests/mutations.rs is `test = false` — it builds a mutated copy of the crate
# per row, so it must not run inside every ordinary `cargo test`.
#
# NAMED `rust-mutations`, not `mutations`, and the reason is a near miss worth
# recording: this recipe and the TLA+ one below were both called `mutations` on
# two branches that had to merge. `just` would have taken one definition and
# the other suite would have vanished from `check` in silence — a whole
# assurance layer deleted by a name collision, which is the exact failure mode
# both suites exist to catch.
rust-mutations:
    cargo test --test mutations -- --nocapture

# --- the formal layer -------------------------------------------------------

# The text half of the proof gate: no `sorry`, no `native_decide`, no `axiom`
# declaration, and every theorem inside the `#print axioms` audit. `lake build`
# exits 0 on all four, so this grep is the only thing between them and a green
# badge. Pure text, milliseconds, no toolchain — which is why it is in `check`
# and in the push hook.
no-sorry:
    scripts/check-lean-proofs.sh

# The bridge. Regenerates spec/tla/lib/RustObs.tla from a real `Explorer::explore`
# run and fails on drift. Needs cargo and nothing else, so it is in `check`:
# a change to src/explore.rs that moves a report counter must not reach CI
# before the model has been asked to agree with it.
model:
    scripts/check-model.sh

# Rewrite spec/tla/lib/RustObs.tla after a deliberate change to the explorer. EXPECT
# TLC to go red on ModelMatchesRust afterwards until the model agrees — that red
# is the point of the gate.
regen-model:
    scripts/check-model.sh --write

# Machine-check every theorem (needs a Lean toolchain; see formal/README.md).
lean:
    cd formal && lake build

# Every proof gate gets a mutation that turns it red, EXECUTED — including the
# one only `#guard_msgs` can catch, where a theorem picks up `Classical.choice`
# from core and no grep can see it. Needs the Lean toolchain.
lean-mutations:
    scripts/check-lean-mutations.sh

# Model-check every green configuration (needs java; check.sh fetches the
# pinned, checksum-verified tla2tools.jar).
tla:
    spec/tla/check.sh
    spec/tla/test-check.sh

# Run every TLA+ mutation and assert the verdict it declares. An invariant with
# no mutation that turns it red is decoration.
tla-mutations:
    scripts/check-mutations.sh

# Everything in the formal layer. Not part of `check` — see the header.
formal: no-sorry lean lean-mutations tla tla-mutations model

# Regenerate every demo GIF from its tape (needs `vhs`).
demos:
    #!/usr/bin/env bash
    set -euo pipefail
    for tape in demos/*.tape; do vhs "$tape"; done

# Regenerate one demo.
demo name:
    vhs demos/{{name}}.tape
