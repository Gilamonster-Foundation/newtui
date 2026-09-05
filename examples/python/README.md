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

panel = newtui.settings_panel(
    settings=[
        newtui.Setting.choice(
            "tenacity",
            "tenacity",
            "auto",
            [
                newtui.Choice("auto", "inherit"),
                newtui.Choice("steady", "persist"),
            ],
        )
    ],
    backend="sol",
    models=["qwen3.5:397b", "nemotron:30b"],
)
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
import newtui


class MyPythonPanel:
    def __init__(self):
        self.level = 0

    def handle(self, key):
        if key == newtui.Key.RIGHT:
            self.level = min(3, self.level + 1)
        elif key == newtui.Key.LEFT:
            self.level = max(0, self.level - 1)
        elif key == newtui.Key.ENTER:
            return newtui.Flow.close(True)
        elif key == newtui.Key.ESC:
            return newtui.Flow.close(False)
        return newtui.Flow.stay()

    def view(self):
        return newtui.View(
            "dial",
            [newtui.Row(
                "level", str(self.level), selected=True, adjustable=True
            )],
            "arrows change",
        )


report = newtui.explore(lambda: MyPythonPanel(), newtui.properties.standard())
assert report.is_clean, report
```

`explore` holds the GIL for the complete walk because every transition can
call Python. A callback exception is recorded in `report.errors`, makes
`report.is_clean` false, and terminates that path; no Python exception is
allowed to unwind across the Rust boundary.

The binding is a separate, non-default workspace member. Build it into an
active virtual environment with `python -m maturin develop --manifest-path
newtui-py/Cargo.toml`; the core crate retains its empty runtime closure.
