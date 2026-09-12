#!/usr/bin/env bash
# Record all named demos, or exactly one checked-in tape.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/capture-common.sh

if (( $# > 1 )); then
  echo 'usage: capture-demos.sh [name]' >&2
  exit 1
fi
if (( $# == 1 )); then
  if [[ ! "$1" =~ ^[a-z0-9_]+$ || ! -f "demos/$1.tape" ]]; then
    echo 'demo must name a checked-in tape in demos/' >&2
    exit 1
  fi
  tapes=("demos/$1.tape")
else
  tapes=(demos/*.tape)
fi

capture_init
capture_build demo --example demo --features ratatui
export NEWTUI_DEMO_BIN="$capture_bin"
capture_record "${tapes[@]}"

captures=()
for tape in "${tapes[@]}"; do
  stem="${tape%.tape}"
  capture_animate "$stem.gif" "$stem.png"
  captures+=("$stem.gif" "$stem.png")
done
capture_publish "${captures[@]}"
for tape in "${tapes[@]}"; do
  name="$(basename "${tape%.tape}")"
  capture_metadata "demos/captures/$name.md" "$tape"
done
echo 'Captured fresh GIF/APNG demos and their reproduction inputs in demos/.'
