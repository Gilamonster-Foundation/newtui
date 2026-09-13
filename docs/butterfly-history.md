# Butterfly history

`newtui::butterfly_history` renders two sampled series as mirrored bars around
a vertical center line. Each row is one sampling interval. Older samples sit
above newer samples; the newest pair always occupies the bottom row.

```rust
let transmit = [10.0, 35.0, 80.0, 50.0];
let receive = [70.0, 40.0, 20.0, 60.0];
let graph = newtui::butterfly_history(&transmit, &receive, 100.0, 61, 20);
assert!(graph.validate(61, 20).is_ok());
```

The host keeps its sample buffers, appends new measurements, and requests a
fresh output when data arrives. The builder has no timer, terminal, sampling
process, or retained history. Both series use the same maximum, so a change
in bar length represents a change in the supplied value. Hosts choose units,
labels, current-rate captions, and the sampling interval.

When the history is longer than the rectangle, the oldest rows are omitted.
Shorter histories leave empty bars above the oldest sample. Unequal slice
lengths align at their newest ends; the host must align actual timestamps
before calling the builder. Empty bars mean empty signal in this display,
including absent or non-finite samples. This API does not interpolate gaps.

Negative values render empty bars; finite values beyond the maximum fill
their side. A zero, negative, or non-finite maximum produces empty signal.
The center remains visible even with no samples. One-column output contains
only the center; zero width and height retain the requested empty rectangle.
With even widths, the left side receives the extra column.

Each row uses the existing compact `newtui::butterfly` glyph ramp and semantic
tones. The compact widget remains useful for a current reading; a host can
show it beside or above the history using the latest pair of samples.

The [live catalog](CATALOG.md) and [recorded demos](../demos/README.md) supply a
deterministic synthetic stream for exploration. Their clocks and pause/step
controls belong to those executable hosts, independently of the library API.
