# Driving newtui from Python

The components are state machines over keys, and their views are plain data —
which makes them drivable from anything that can call a function and read a
record, not just from Rust.

The Python face lands as a separate, non-default workspace member (`newtui-py`,
PyO3), so a plain `cargo build` never compiles it — the same arrangement
`precedence-ladder` uses for its own Python face.

## What it is for

Two uses, and they are different:

**Building a TUI in Python.** Drive a component, get a view back, render it with
whatever you like — Textual, Rich, blessed, or your own writer. The component
owns the behaviour; your host owns the drawing.

```python
import newtui

panel = newtui.settings_panel(backend="sol", models=["qwen3.5:397b", "nemotron:30b"])
panel.handle(newtui.Key.DOWN)
panel.handle(newtui.Key.RIGHT)

view = panel.view()
for row in view.rows:
    mark = ">" if row.selected else " "
    print(f"{mark} {row.label:<28} {row.value}")
```

**Holding a Python reimplementation to the same corpus.** The acceptance
properties are claims about observable behaviour, so a component written in
Python can be explored and judged by exactly the set that judges the Rust one.
That is the part worth the binding: a shared corpus is how two implementations
of one component stay one component.

```python
report = newtui.explore(lambda: MyPythonPanel(), newtui.properties.standard())
assert report.is_clean, report
```

## Status

Scaffolding. The seam is settled and the Rust side is real; the binding lands
once the first components move over, so it can be shaped by what they actually
export rather than by a guess.
