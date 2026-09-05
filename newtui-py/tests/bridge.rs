use std::ffi::CString;

use pyo3::prelude::*;
use pyo3::types::PyDict;

fn with_module(code: &str) {
    Python::initialize();
    Python::attach(|py| {
        let module = PyModule::new(py, "_native").expect("a module");
        _native::add_to_module(&module).expect("the binding registers");
        PyModule::import(py, "sys")
            .expect("sys is importable")
            .getattr("modules")
            .expect("sys.modules exists")
            .set_item("newtui", &module)
            .expect("the local binding is importable as newtui");
        let locals = PyDict::new(py);
        locals.set_item("newtui", module).expect("a local module");
        let code = CString::new(code).expect("test code contains no NUL");
        py.run(&code, Some(&locals), Some(&locals))
            .unwrap_or_else(|error| panic!("Python test failed: {error}"));
    });
}

fn python_blocks(markdown: &str) -> impl Iterator<Item = &str> {
    markdown
        .split("```python\n")
        .skip(1)
        .map(|tail| tail.split_once("```").expect("a closed Python fence").0)
}

// GUARD: python_documentation_examples_run — this is a guard; tests/mutations.rs must show it red.
#[test]
fn python_documentation_examples_run() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the binding is a workspace member");
    let paths = [
        root.join("examples/python/README.md"),
        root.join("docs/CATALOG.md"),
    ];
    let mut checked = 0;
    for path in paths {
        let markdown = std::fs::read_to_string(&path).expect("the documentation is readable");
        for block in python_blocks(&markdown) {
            with_module(block);
            checked += 1;
        }
    }
    assert!(checked > 0, "the documentation scan executed nothing");
}

#[test]
fn python_drives_the_rust_component_as_plain_objects() {
    with_module(
        r#"
choice = newtui.Choice("auto", "inherit")
other = newtui.Choice("steady", "persist")
setting = newtui.Setting.choice("tenacity", "tenacity", "auto", [choice, other])
number = newtui.Setting.number("rounds", "round limit", "auto", "auto", 1, 4)
fixed = newtui.Setting.fixed("prompt", "prompt", "> ", "host editor")
panel = newtui.settings_panel(
    settings=[setting, number, fixed],
    backend="sol",
    models=["qwen", "nemotron"],
    current_model="qwen",
)
assert repr(newtui.Key.RIGHT) == "Key(Right)"
assert repr(newtui.Flow.stay()) == "Flow.stay()"
assert repr(newtui.Flow.close(True)) == "Flow.close(true)"
assert not newtui.Flow.stay().closed
assert newtui.Flow.close(True).closed
assert newtui.Flow.close(True).applied
assert not newtui.Flow.close(False).applied
assert newtui.Key.character("x") != newtui.Key.control("x")

panel.handle(newtui.Key.RIGHT)
panel.handle(newtui.Key.DOWN)
panel.handle(newtui.Key.RIGHT)
view = panel.view()
assert view.title == "settings"
assert view.footer
assert view.rows[0].label == "tenacity"
assert view.rows[0].value == "steady"
assert view.rows[0].note.startswith("was ")
assert view.rows[1].selected and view.rows[1].adjustable
panel.handle(newtui.Key.ENTER)
intent = panel.intent()
assert intent.kind == "apply"
assert intent.model is None
assert [(change.key, change.value) for change in intent.changes] == [
    ("tenacity", "steady"), ("rounds", "1")
]
"#,
    );
}

#[test]
fn rust_explores_python_with_and_without_an_explicit_fingerprint() {
    with_module(
        r#"
class Dial:
    def __init__(self):
        self.level = 0
    def handle(self, key):
        if key == newtui.Key.RIGHT:
            self.level = min(3, self.level + 1)
            return newtui.Flow.stay()
        if key == newtui.Key.LEFT:
            self.level = max(0, self.level - 1)
            return newtui.Flow.stay()
        if key == newtui.Key.ENTER:
            return newtui.Flow.close(True)
        if key == newtui.Key.ESC:
            return newtui.Flow.close(False)
        return newtui.Flow.stay()
    def view(self):
        return newtui.View("dial", [newtui.Row(
            "level", str(self.level), selected=True, adjustable=True
        )], "arrows")

report = newtui.explore(lambda: Dial(), newtui.properties.standard())
assert report.is_clean, str(report)
assert report.verdict.kind == "clean" and report.verdict.reason is None
assert report.exhausted and report.states == 4 and report.transitions > 0
assert report.errors == []
assert report.properties and all(p.outcome == "held" for p in report.properties)

class Explicit(Dial):
    def fingerprint(self):
        return f"level={self.level}"

explicit = newtui.explore(lambda: Explicit(), newtui.properties.standard())
assert explicit.is_clean, str(explicit)
"#,
    );
}

// GUARD: python_report_preserves_three_valued_verdict — this is a guard; tests/mutations.rs must show it red.
#[test]
fn python_report_preserves_three_valued_verdict() {
    with_module(
        r#"
class Dial:
    def __init__(self, level=0):
        self.level = level
    def handle(self, key):
        if key == newtui.Key.ENTER:
            return newtui.Flow.close(True)
        if key == newtui.Key.ESC:
            return newtui.Flow.close(False)
        return newtui.Flow.stay()
    def view(self):
        return newtui.View("dial", [newtui.Row(
            "level", str(self.level), selected=True, adjustable=True
        )], "")
    def fingerprint(self):
        return str(self.level)

clean = newtui.explore(lambda: Dial(), newtui.properties.standard())
assert clean.verdict.kind == "clean" and clean.verdict.reason is None
assert all(prop.outcome == "held" for prop in clean.properties)

class Broken(Dial):
    def view(self):
        return newtui.View("dial", [
            newtui.Row("level", "0", selected=True, adjustable=True),
            newtui.Row("also selected", "x", selected=True, adjustable=True),
        ], "")

violated = newtui.explore(lambda: Broken(), newtui.properties.standard())
assert violated.verdict.kind == "violated"
assert any(prop.outcome == "violated" for prop in violated.properties)

builds = 0
def drifting():
    global builds
    component = Dial(builds)
    builds += 1
    return component

incomplete = newtui.explore(drifting, newtui.properties.standard())
assert incomplete.verdict.kind == "incomplete"
assert "REPLAY DID NOT LAND" in incomplete.verdict.reason

class Empty(Dial):
    def view(self):
        return newtui.View("empty", [], "")

unreached = newtui.explore(lambda: Empty(), newtui.properties.standard())
assert unreached.verdict.kind == "incomplete"
assert "PROPERTY NEVER APPLIED" in unreached.verdict.reason
assert any(prop.outcome == "not_applicable" for prop in unreached.properties)
"#,
    );
}

// GUARD: python_callback_exceptions_are_reported — this is a guard; tests/mutations.rs must show it red.
#[test]
fn python_callback_exceptions_are_reported() {
    with_module(
        r#"
class Panel:
    def handle(self, key):
        raise RuntimeError("dial broke")
    def view(self):
        return newtui.View("panel", [newtui.Row("x", "0", selected=True)], "")

handle = newtui.explore(lambda: Panel(), newtui.properties.standard())
assert not handle.is_clean
assert handle.verdict.kind == "incomplete"
assert "PYTHON CALLBACK FAILED" in handle.verdict.reason
assert handle.errors[0].method == "handle"
assert "RuntimeError: dial broke" in handle.errors[0].detail
assert handle.errors[0].path
assert "Python callback errors" in str(handle)

class BadView(Panel):
    def handle(self, key):
        return newtui.Flow.stay()
    def view(self):
        raise ValueError("no view")

view = newtui.explore(lambda: BadView(), newtui.properties.standard())
assert not view.is_clean and view.errors[0].method == "view"

class BadFingerprint(BadView):
    def view(self):
        return newtui.View("panel", [newtui.Row("x", "0", selected=True)], "")
    def fingerprint(self):
        raise LookupError("no identity")

fingerprint = newtui.explore(
    lambda: BadFingerprint(), newtui.properties.standard()
)
assert not fingerprint.is_clean
assert fingerprint.errors[0].method == "fingerprint"

def bad_factory():
    raise LookupError("no component")

factory = newtui.explore(bad_factory, newtui.properties.standard())
assert not factory.is_clean and factory.errors[0].method == "factory"
"#,
    );
}
