#!/usr/bin/env bash
# Reproducible, FAIL-CLOSED TLC runner for the behavioral constitution (#1529).
#
# Resolves a PINNED, checksum-verified tla2tools.jar, then model-checks the root
# models named in `models.txt`. Invariants (deliberately fail-closed — a green
# exit must mean specs were actually checked):
#   * `models.txt` is REQUIRED and must name at least one model;
#   * every manifest entry must have BOTH <Name>.tla and <Name>.cfg;
#   * every root <Name>.cfg must be named in the manifest;
#   * every root <Name>.tla must be named in the manifest;
#   * an explicitly-requested spec must be named in the manifest;
#   * the number of specs CHECKED must equal the number requested;
#   * a TLC failure is an ERROR; a skipped spec never yields success;
#   * unsafe spec names (path separators / traversal) are rejected;
#   * the jar checksum is enforced for EVERY source (env / local / cache /
#     download); a locally-supplied jar of another version is refused, not run.
#
# THE MANIFEST REPLACED A `*.cfg` GLOB, and that is not a style change. Under the
# glob, a `.tla` whose `.cfg` was never written was skipped SILENTLY and the run
# went green on a count that did not include it — the false-completeness class
# this spec directory exists to pin, in the tooling used to pin it. The naive
# repair ("every .tla needs a .cfg") is wrong, because an imported support module
# legitimately has none; so support modules live in `lib/` and are excluded BY
# LOCATION. See models.txt for the full statement and test-check.sh for the
# expected-red row behind each of the four validations.
#
# This runner has therefore FORKED from newt-agent's copy, which still globs.
# The divergence is deliberate and this paragraph is the record of it.
#
# Usage:  spec/tla/check.sh [Spec ...]     # default: every model in models.txt
# Env:    TLA2TOOLS_JAR  — explicit jar (still checksum-enforced)
#         TLA_SPEC_DIR   — spec directory (default: this script's dir; for tests)
set -euo pipefail

# ── Pin (bump deliberately; update the checksum in lock-step) ────────────────
readonly TLA2TOOLS_VERSION="1.7.4"
readonly TLA2TOOLS_SHA256="936a262061c914694dfd669a543be24573c45d5aa0ff20a8b96b23d01e050e88"
readonly TLA2TOOLS_URL="https://github.com/tlaplus/tlaplus/releases/download/v${TLA2TOOLS_VERSION}/tla2tools.jar"

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
spec_dir="${TLA_SPEC_DIR:-$here}"
cache="${XDG_CACHE_HOME:-$HOME/.cache}/newt-tla"
readonly cached_jar="$cache/tla2tools-${TLA2TOOLS_VERSION}.jar"

log() { printf '[tla] %s\n' "$*" >&2; }
die() { log "ERROR: $*"; exit "${2:-2}"; }

# ── Portable sha256 (sha256sum OR shasum -a 256) ────────────────────────────
sha256_hex() {
  if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then shasum -a 256 "$1" | awk '{print $1}'
  else die "no sha256 tool found (need 'sha256sum' or 'shasum -a 256')" 3; fi
}
matches_pin() { [ "$(sha256_hex "$1")" = "$TLA2TOOLS_SHA256" ]; }

# ── Atomic, checksum-verified download into the cache ───────────────────────
download_jar() {
  mkdir -p "$cache"
  local tmp
  tmp="$(mktemp "$cache/.tla2tools.XXXXXX")"
  # Clean the partial on ANY exit from this function (SIGINT/failure included).
  trap 'rm -f "$tmp"' RETURN
  log "fetching pinned tla2tools ${TLA2TOOLS_VERSION} …"
  command -v curl >/dev/null 2>&1 || die "curl not found; cannot fetch tla2tools" 2
  curl -fsSL "$TLA2TOOLS_URL" -o "$tmp" || die "download failed from $TLA2TOOLS_URL"
  matches_pin "$tmp" || die "downloaded jar checksum MISMATCH — refusing"
  # Atomic publish: a concurrent reader sees either the old jar or the fully
  # verified new one, never a partial.
  mv -f "$tmp" "$cached_jar"
  trap - RETURN
  printf '%s\n' "$cached_jar"
}

resolve_jar() {
  # Sweep any orphaned partials from an interrupted prior run.
  rm -f "$cache"/.tla2tools.* 2>/dev/null || true

  # 1) $TLA2TOOLS_JAR — explicit, so a wrong-version jar is a HARD failure
  #    (never silently accept another version).
  if [ -n "${TLA2TOOLS_JAR:-}" ]; then
    [ -f "$TLA2TOOLS_JAR" ] || die "\$TLA2TOOLS_JAR=$TLA2TOOLS_JAR does not exist"
    matches_pin "$TLA2TOOLS_JAR" \
      || die "\$TLA2TOOLS_JAR is not the pinned tla2tools ${TLA2TOOLS_VERSION} (checksum mismatch) — refusing"
    printf '%s\n' "$TLA2TOOLS_JAR"; return 0
  fi
  # 2) conventional local install (soft: ignore if mismatched)
  local local_install="$HOME/opt/tla2tools/tla2tools.jar"
  if [ -f "$local_install" ] && matches_pin "$local_install"; then
    printf '%s\n' "$local_install"; return 0
  fi
  # 3) cache (reject + remove a corrupt cached jar, then re-download)
  if [ -f "$cached_jar" ]; then
    if matches_pin "$cached_jar"; then printf '%s\n' "$cached_jar"; return 0; fi
    log "cached jar failed checksum — discarding and re-fetching"
    rm -f "$cached_jar"
  fi
  # 4) atomic, verified download
  download_jar
}

# ── The root-model manifest, validated in four directions (fail-closed) ─────
readonly manifest="$spec_dir/models.txt"
[ -f "$manifest" ] || die "no root-model manifest at $manifest (see spec/tla/models.txt)"

safe_name() {
  case "$1" in
    "" | */* | *\\* | .. | *..*) die "invalid spec name: '$1' (no path separators or traversal)" ;;
  esac
}

listed() {
  local want="$1" n
  for n in "${models[@]}"; do [ "$n" = "$want" ] && return 0; done
  return 1
}

models=()
while IFS= read -r line; do
  line="${line%%#*}"                       # strip comments
  line="${line#"${line%%[![:space:]]*}"}"  # ltrim
  line="${line%"${line##*[![:space:]]}"}"  # rtrim
  [ -n "$line" ] || continue
  safe_name "$line"
  listed "$line" && die "models.txt lists '$line' twice"
  models+=("$line")
done < "$manifest"

[ "${#models[@]}" -gt 0 ] || die "models.txt names no models — nothing would be checked"

# (1) every manifest entry is a complete pair.
for name in "${models[@]}"; do
  [ -f "$spec_dir/$name.tla" ] || die "models.txt lists '$name' but $spec_dir/$name.tla does not exist"
  [ -f "$spec_dir/$name.cfg" ] || die "models.txt lists '$name' but $spec_dir/$name.cfg does not exist"
done

# (2) and (3): every root .cfg and every root .tla is listed. NOT recursive —
# lib/ holds the imported support modules and is excluded by LOCATION, which is
# the whole reason a manifest beats an "every .tla needs a .cfg" rule.
shopt -s nullglob
for f in "$spec_dir"/*.cfg "$spec_dir"/*.tla; do
  name="$(basename "$f")"; name="${name%.*}"
  listed "$name" || die \
    "$(basename "$f") is not named in models.txt. A root model must be listed; an imported support module belongs in $spec_dir/lib/."
done
shopt -u nullglob

# ── Collect specs (fail-closed) ─────────────────────────────────────────────
specs=()
if [ "$#" -gt 0 ]; then
  for s in "$@"; do
    safe_name "$s"
    listed "$s" || die "requested spec '$s' is not named in models.txt"
    specs+=("$s")
  done
else
  specs=("${models[@]}")
fi

jar="$(resolve_jar)"
[ -n "$jar" ] && [ -f "$jar" ] || die "could not resolve a verified tla2tools.jar"
log "using tla2tools ${TLA2TOOLS_VERSION}: $jar"

# ── Check each spec; a failure or a skip must NOT produce overall success ───
# `lib/` is on TLC's module search path so an imported support module resolves
# from the place that excludes it from discovery.
tla_lib=()
[ -d "$spec_dir/lib" ] && tla_lib=(-DTLA-Library=lib)

checked=0
for spec in "${specs[@]}"; do
  log "TLC checking ${spec}.tla …"
  # -XX:+UseParallelGC is TLC's recommended GC; run inside the spec dir.
  # ${a[@]+"${a[@]}"} — an empty array under `set -u` is an error before bash 4.4.
  ( cd "$spec_dir" && java -XX:+UseParallelGC ${tla_lib[@]+"${tla_lib[@]}"} \
      -cp "$jar" tlc2.TLC -config "${spec}.cfg" "${spec}.tla" ) \
    || die "TLC FAILED on ${spec}" 1
  checked=$((checked + 1))
done

# (4) THE ASSERTION THAT CONVERTS AN OMISSION FROM A SILENT GREEN INTO A
# FAILURE. Everything above validates the inputs; this validates the outcome.
# It is the one check that would still catch a discovery bug reintroduced
# below the manifest — including a `continue` slipped into the loop above.
[ "$checked" -eq "${#specs[@]}" ] \
  || die "checked $checked specification(s) but $((${#specs[@]})) were requested — a spec was skipped"
[ "$checked" -gt 0 ] || die "no specifications were checked"
log "OK — ${checked} specification(s) checked with tla2tools ${TLA2TOOLS_VERSION} (TLC)."
printf 'tla-checked-count=%d\n' "$checked"
