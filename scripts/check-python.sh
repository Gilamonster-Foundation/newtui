#!/usr/bin/env bash
# The Python half of `just check`; mirrored by ci.yml and the pre-push fallback.
set -euo pipefail

if [[ -n "${PYTHON:-}" ]]; then
  python_bin="$PYTHON"
elif [[ -x .venv/bin/python ]]; then
  python_bin=.venv/bin/python
else
  python_bin=python3
fi
python_prefix="$($python_bin -c 'import sys; print(sys.prefix)')"

# maturin needs to know which interpreter owns the editable extension even
# when the caller names a venv's Python without activating that venv.
VIRTUAL_ENV="$python_prefix" PYO3_PYTHON="$python_bin" \
  "$python_bin" -m maturin develop --manifest-path newtui-py/Cargo.toml

coverage_file="target/python.coverage"
"$python_bin" -m coverage erase --data-file="$coverage_file"
"$python_bin" -m coverage run \
  --data-file="$coverage_file" \
  --branch \
  --source=newtui-py/python/newtui \
  -m unittest discover -s newtui-py/python-tests -v
"$python_bin" -m coverage report \
  --data-file="$coverage_file" \
  --fail-under=80 \
  --show-missing
