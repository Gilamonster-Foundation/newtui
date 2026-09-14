#!/usr/bin/env bash
# Record the real catalog host with fixed fixtures and checked-in VHS tapes.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/capture-common.sh
capture_init
capture_build newtui-catalog -p newtui-catalog
export NEWTUI_CATALOG_BIN="$capture_bin"
tapes=(demos/catalog/*.tape)
capture_record "${tapes[@]}"

captures=(
  docs/widgets/generated/catalog.png
  docs/widgets/generated/butterfly.png
  docs/widgets/generated/heat-meter.png
  docs/widgets/generated/gauge.png
  docs/widgets/generated/catalog-light.png
  docs/widgets/generated/catalog-narrow.png
  docs/widgets/generated/catalog-error.png
  docs/widgets/generated/diff-unified.png
  docs/widgets/generated/diff-split.png
  docs/widgets/generated/diff-stat.png
  docs/widgets/generated/diff-unicode.png
  docs/widgets/generated/diff-tiny.png
  docs/widgets/generated/bsp.png
  docs/widgets/generated/bsp-narrow.png
  docs/widgets/generated/bsp-error.png
  docs/widgets/generated/linked-panes.png
  docs/widgets/generated/linked-panes-anchor.png
  docs/widgets/generated/linked-panes-narrow.png
  docs/widgets/generated/butterfly-history.png
  docs/widgets/generated/core-grid-live.png
  demos/catalog/catalog.gif
  demos/catalog/light.gif
  demos/catalog/narrow.gif
  demos/catalog/error.gif
  demos/catalog/diff.gif
  demos/catalog/bsp.gif
  demos/catalog/linked_panes.gif
  demos/catalog/activity.gif
)

capture_animate demos/catalog/catalog.gif demos/catalog/catalog.png
captures+=(demos/catalog/catalog.png)
capture_publish "${captures[@]}"
capture_metadata docs/widgets/generated/CAPTURES.md "${tapes[@]}"
capture_metadata docs/widgets/generated/BSP-CAPTURES.md demos/catalog/bsp.tape
capture_metadata docs/widgets/generated/LINKED-PANES-CAPTURES.md demos/catalog/linked_panes.tape
capture_metadata docs/widgets/generated/ACTIVITY-CAPTURES.md demos/catalog/activity.tape
echo 'Captured catalog PNGs in docs/widgets/generated and GIF/APNG in demos/catalog.'
