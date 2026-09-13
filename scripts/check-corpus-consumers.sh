#!/usr/bin/env bash
# Native Python and Go consumers of the identical Rust-exported fixture.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ -n "${PYTHON:-}" ]]; then
  python_bin="$PYTHON"
elif [[ -x .venv/bin/python ]]; then
  python_bin=.venv/bin/python
else
  python_bin=python3
fi
mkdir -p target/corpus
"$python_bin" -m coverage run \
  --data-file=target/corpus/python.coverage \
  --source=newtui-corpus/python \
  -m unittest discover -s newtui-corpus/python -p 'test_*.py'
"$python_bin" -m coverage report \
  --data-file=target/corpus/python.coverage \
  --include='*/consumer.py' --show-missing --fail-under=80
"$python_bin" newtui-corpus/python/consumer.py newtui-corpus/fixtures/dial.json
(
  cd newtui-corpus/go
  if [[ -n "$(gofmt -l .)" ]]; then
    echo 'corpus: Go source is not formatted' >&2
    exit 1
  fi
  go vet ./...
  go test -count=1 -coverprofile=../../target/corpus/go.coverage ./...
  go tool cover -func=../../target/corpus/go.coverage > ../../target/corpus/go-summary.txt
  awk '/^total:/ { seen = 1; value = $3; sub(/%$/, "", value); print; if (value + 0 < 80) exit 1 } END { if (!seen) exit 1 }' ../../target/corpus/go-summary.txt
  go run . ../fixtures/dial.json
)
