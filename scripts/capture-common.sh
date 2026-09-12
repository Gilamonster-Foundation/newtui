#!/usr/bin/env bash
# Shared by the demo and catalog recorders. Source from the repository root.

capture_init() {
  repo_root="$PWD"
  recorder="${VHS_BIN:-vhs}"
  for dependency in cargo python3 "$recorder" ffmpeg ffprobe; do
    command -v "$dependency" >/dev/null || {
      echo "capture requires $dependency" >&2
      exit 1
    }
  done
  # Recording changes cwd; a relative VHS_BIN must keep referring to the
  # executable selected from the repository root, including paths with spaces.
  recorder="$(command -v "$recorder")"
  if [[ "$recorder" != /* ]]; then
    recorder="$repo_root/$recorder"
  fi
  report="$(mktemp "${TMPDIR:-/tmp}/newtui-capture-build.XXXXXX")"
  capture_dir="$(mktemp -d "${TMPDIR:-/tmp}/newtui-capture.XXXXXX")"
  trap 'rm -f "$report"; rm -rf "$capture_dir"' EXIT
  mkdir -p "$capture_dir/docs/widgets/generated" "$capture_dir/demos/catalog"
}

capture_build() {
  local target_name="$1"
  shift
  cargo build --locked --message-format=json "$@" >"$report"
  capture_bin="$(python3 - "$report" "$target_name" <<'PY'
import json
import sys

executables = []
with open(sys.argv[1], encoding="utf-8") as messages:
    for line in messages:
        message = json.loads(line)
        if (message.get("reason") == "compiler-artifact"
                and message.get("target", {}).get("name") == sys.argv[2]
                and message.get("executable")):
            executables.append(message["executable"])
if len(executables) != 1:
    raise SystemExit("cargo did not report exactly one requested executable")
print(executables[0])
PY
)"
}

capture_record() (
  cd "$capture_dir"
  unset NO_COLOR
  export TERM=xterm-256color COLORTERM=truecolor
  for tape in "$@"; do
    "$recorder" "$repo_root/$tape"
  done
)

capture_animate() {
  local gif="$1" png="$2" frames
  frames="$(ffprobe -v error -count_frames -select_streams v:0 \
    -show_entries stream=nb_read_frames -of default=noprint_wrappers=1:nokey=1 \
    "$capture_dir/$gif")"
  if [[ ! "$frames" =~ ^[0-9]+$ ]] || (( frames < 2 )); then
    echo "recording must contain multiple frames: $gif; existing captures were preserved" >&2
    exit 1
  fi
  ffmpeg -hide_banner -loglevel error -xerror -y -i "$capture_dir/$gif" \
    -plays 0 -f apng "$capture_dir/$png"
}

capture_publish() {
  local capture
  # Validate the entire fresh set before replacing any checked-in artifact.
  for capture in "$@"; do
    if [[ ! -s "$capture_dir/$capture" ]]; then
      echo "recorder did not create $capture; existing captures were preserved" >&2
      exit 1
    fi
    ffmpeg -hide_banner -loglevel error -xerror -i "$capture_dir/$capture" -f null -
  done
  for capture in "$@"; do
    mkdir -p "$(dirname "$capture")"
    cp "$capture_dir/$capture" "$capture"
  done
}

capture_metadata() {
  local destination="$1" tape
  shift
  mkdir -p "$(dirname "$destination")"
  {
    echo '# Capture reproduction inputs'
    echo
    echo 'Generated from the real terminal host using the checked-in tapes below.'
    echo
    echo "Source revision: $(git rev-parse HEAD)"
    echo "Recorder: $("$recorder" --version)"
    if [[ -n "$(git status --porcelain -- .)" ]]; then
      echo 'Working tree: includes the reviewed changes accompanying these captures.'
    fi
    for tape in "$@"; do
      echo
      echo "## $tape"
      echo
      echo '```text'
      cat "$tape"
      echo '```'
    done
  } >"$destination"
}
