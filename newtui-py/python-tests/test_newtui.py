import unittest

import newtui


class PythonDial:
    def __init__(self):
        self.level = 0

    def handle(self, key):
        if key == newtui.Key.RIGHT:
            self.level = min(self.level + 1, 3)
            return newtui.Flow.stay()
        if key == newtui.Key.LEFT:
            self.level = max(self.level - 1, 0)
            return newtui.Flow.stay()
        if key == newtui.Key.ENTER:
            return newtui.Flow.close(True)
        if key == newtui.Key.ESC:
            return newtui.Flow.close(False)
        return newtui.Flow.stay()

    def view(self):
        return newtui.View(
            "python dial",
            [newtui.Row("level", str(self.level), selected=True, adjustable=True)],
            "arrows change",
        )

    def fingerprint(self):
        return f"level={self.level}"


class NewtuiTests(unittest.TestCase):
    def test_python_drives_the_rust_settings_panel(self):
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
                ),
                newtui.Setting.number(
                    "rounds", "round limit", "auto", "auto", 1, 4
                ),
                newtui.Setting.fixed(
                    "prompt", "input prompt", "> ", "edit in the host"
                ),
            ],
            backend="sol",
            models=["qwen", "nemotron"],
            current_model="qwen",
        )

        self.assertEqual(repr(newtui.Key.OTHER), "Key(Other)")
        self.assertNotEqual(newtui.Key.OTHER, newtui.Key.ESC)
        self.assertEqual(panel.handle(newtui.Key.OTHER), newtui.Flow.stay())
        self.assertEqual(panel.view().rows[0].value, "auto")
        panel.handle(newtui.Key.RIGHT)
        panel.handle(newtui.Key.DOWN)
        panel.handle(newtui.Key.RIGHT)
        view = panel.view()

        self.assertEqual(view.title, "settings")
        self.assertEqual(view.rows[0].value, "steady")
        self.assertEqual(view.rows[1].value, "1")
        self.assertTrue(view.rows[1].selected)
        self.assertEqual(panel.handle(newtui.Key.ENTER), newtui.Flow.close(True))
        intent = panel.intent()
        self.assertEqual(intent.kind, "apply")
        self.assertEqual(
            [(change.key, change.value) for change in intent.changes],
            [("tenacity", "steady"), ("rounds", "1")],
        )

    def test_rust_explores_a_python_component(self):
        report = newtui.explore(
            lambda: PythonDial(), newtui.properties.standard()
        )

        self.assertTrue(report.exhausted)
        self.assertTrue(report.is_clean, str(report))
        self.assertEqual(report.verdict.kind, "clean")
        self.assertIsNone(report.verdict.reason)
        self.assertEqual(report.states, 4)
        self.assertGreater(report.transitions, 0)
        self.assertEqual(report.errors, [])
        self.assertTrue(report.properties)
        self.assertTrue(
            all(prop.outcome == "held" for prop in report.properties)
        )

    def test_verdict_distinguishes_violation_from_incomplete(self):
        class BrokenSelection(PythonDial):
            def view(self):
                return newtui.View(
                    "broken",
                    [
                        newtui.Row(
                            "level",
                            str(self.level),
                            selected=True,
                            adjustable=True,
                        ),
                        newtui.Row(
                            "also selected", "x", selected=True, adjustable=True
                        ),
                    ],
                    "",
                )

        violated = newtui.explore(
            lambda: BrokenSelection(), newtui.properties.standard()
        )
        self.assertEqual(violated.verdict.kind, "violated")
        self.assertIsNone(violated.verdict.reason)
        self.assertTrue(
            any(prop.outcome == "violated" for prop in violated.properties)
        )

        builds = 0

        def drifting_factory():
            nonlocal builds
            component = PythonDial()
            component.level = builds
            builds += 1
            return component

        incomplete = newtui.explore(
            drifting_factory, newtui.properties.standard()
        )
        self.assertEqual(incomplete.verdict.kind, "incomplete")
        self.assertIn("REPLAY DID NOT LAND", incomplete.verdict.reason)

        class Empty(PythonDial):
            def view(self):
                return newtui.View("empty", [], "")

        unreached = newtui.explore(
            lambda: Empty(), newtui.properties.standard()
        )
        self.assertEqual(unreached.verdict.kind, "incomplete")
        self.assertIn("PROPERTY NEVER APPLIED", unreached.verdict.reason)
        self.assertTrue(
            any(
                prop.outcome == "not_applicable"
                for prop in unreached.properties
            )
        )

    def test_handle_exception_becomes_a_reported_error(self):
        class Explodes(PythonDial):
            def handle(self, key):
                raise RuntimeError("dial broke")

        report = newtui.explore(
            lambda: Explodes(), newtui.properties.standard()
        )

        self.assertFalse(report.is_clean)
        self.assertEqual(report.verdict.kind, "incomplete")
        self.assertIn("PYTHON CALLBACK FAILED", report.verdict.reason)
        self.assertEqual(report.errors[0].method, "handle")
        self.assertIn("RuntimeError: dial broke", report.errors[0].detail)
        self.assertIn("Python callback errors", str(report))

    def test_view_and_factory_exceptions_are_reported_too(self):
        class BadView(PythonDial):
            def view(self):
                raise ValueError("no view")

        bad_view = newtui.explore(
            lambda: BadView(), newtui.properties.standard()
        )
        self.assertFalse(bad_view.is_clean)
        self.assertEqual(bad_view.errors[0].method, "view")

        def bad_factory():
            raise LookupError("no component")

        bad_build = newtui.explore(
            bad_factory, newtui.properties.standard()
        )
        self.assertFalse(bad_build.is_clean)
        self.assertEqual(bad_build.errors[0].method, "factory")


if __name__ == "__main__":
    unittest.main()
