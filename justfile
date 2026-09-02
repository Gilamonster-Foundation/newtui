# justfile for newtui.
#
# `just check` is the full local gate and mirrors .github/workflows/ci.yml.
# Keep the three in step: this file, that workflow, and .githooks/pre-push.

# Format, lint, test and document — the whole gate.
check: fmt clippy test doc leaf

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

# Regenerate every demo GIF from its tape (needs `vhs`).
demos:
    #!/usr/bin/env bash
    set -euo pipefail
    for tape in demos/*.tape; do vhs "$tape"; done

# Regenerate one demo.
demo name:
    vhs demos/{{name}}.tape
