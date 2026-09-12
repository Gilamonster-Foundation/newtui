"""The recorder boundary must preserve prior artifacts when fresh capture fails.

These tests execute the real shell wrappers with fake external tools. They prove
publication ordering, executable/environment wiring and failure handling; real
VHS runs and decoded/visually reviewed frames establish the media itself.
"""

import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


FAKE_TOOL = r'''#!/usr/bin/env python3
import json
import os
from pathlib import Path
import sys

tool = Path(sys.argv[0]).name
args = sys.argv[1:]
root = Path(os.environ["CAPTURE_TEST_ROOT"])
with (root / "calls.jsonl").open("a") as log:
    log.write(json.dumps([tool, args]) + "\n")
if tool == "cargo":
    message = {"reason": "compiler-artifact", "target": {"name": "demo"},
               "executable": str(root / "build with spaces" / "actual-demo")}
    print(json.dumps(message))
    if os.environ.get("AMBIGUOUS_BINARY"):
        print(json.dumps(message))
elif tool == "selected-vhs":
    if args == ["--version"]:
        print("test-recorder")
    else:
        assert "NO_COLOR" not in os.environ
        assert os.environ["TERM"] == "xterm-256color"
        assert os.environ["COLORTERM"] == "truecolor"
        assert os.environ["NEWTUI_DEMO_BIN"] == str(root / "build with spaces" / "actual-demo")
        name = Path(args[0]).stem
        if name != os.environ.get("MISSING_TAPE"):
            Path(f"demos/{name}.gif").write_text(f"fresh gif {name}")
elif tool == "ffprobe":
    assert Path(args[-1]).is_file(), "must inspect the fresh recording"
    print(1 if os.environ.get("SINGLE_FRAME") else 2)
elif tool == "ffmpeg":
    source = Path(args[args.index("-i") + 1])
    assert source.read_text().startswith("fresh "), "never consume checked-in stale output"
    if args[-1] != "-":
        if os.environ.get("FAIL_CONVERSION") and source.stem == "b":
            sys.exit(2)
        Path(args[-1]).write_text(f"fresh apng {source.stem}")
    elif os.environ.get("CORRUPT_OUTPUT") and source.name == "b.png":
        sys.exit(3)
elif tool == "git":
    if args[:2] == ["rev-parse", "HEAD"]:
        print("verified-source-revision")
else:
    raise AssertionError(f"unselected tool {tool}")
'''


class CaptureTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="newtui-capture-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name) / "checkout with spaces"
        shutil.copytree(Path(__file__).parent, self.root / "scripts")
        (self.root / "demos").mkdir()
        for name in ("a", "b"):
            (self.root / "demos" / f"{name}.tape").write_text(f"Output demos/{name}.gif\n")
            for extension in ("gif", "png"):
                (self.root / "demos" / f"{name}.{extension}").write_text("prior capture")
        self.bin = self.root / "fake tools"
        self.bin.mkdir()
        for name in ("cargo", "selected-vhs", "ffmpeg", "ffprobe", "git"):
            executable = self.bin / name
            executable.write_text(FAKE_TOOL)
            executable.chmod(0o755)

    def capture(self, *arguments, **overrides):
        env = dict(os.environ, PATH=f"{self.bin}{os.pathsep}{os.environ['PATH']}",
                   CAPTURE_TEST_ROOT=str(self.root), NO_COLOR="1", TERM="dumb",
                   VHS_BIN=str(self.bin / "selected-vhs"),
                   CARGO_TARGET_DIR=str(self.root / "build with spaces"))
        env.update(overrides)
        return subprocess.run(
            ["bash", str(self.root / "scripts/capture-demos.sh"), *arguments],
            cwd=self.temporary.name, env=env, capture_output=True, text=True,
            check=False,
        )

    def assert_prior_captures(self):
        for name in ("a", "b"):
            for extension in ("gif", "png"):
                self.assertEqual((self.root / "demos" / f"{name}.{extension}").read_text(),
                                 "prior capture")
        self.assertFalse((self.root / "demos/captures").exists())

    def test_all_demos_use_cargo_executable_and_publish_only_after_validation(self):
        result = self.capture()
        self.assertEqual(result.returncode, 0, result.stderr)
        calls = [json.loads(line) for line in (self.root / "calls.jsonl").read_text().splitlines()]
        self.assertEqual(sum(tool == "cargo" for tool, _ in calls), 1)
        for name in ("a", "b"):
            self.assertEqual((self.root / "demos" / f"{name}.gif").read_text(), f"fresh gif {name}")
            self.assertEqual((self.root / "demos" / f"{name}.png").read_text(), f"fresh apng {name}")
            note = (self.root / "demos/captures" / f"{name}.md").read_text()
            self.assertIn("verified-source-revision", note)
            self.assertIn(f"Output demos/{name}.gif", note)

    def test_selected_demo_preserves_other_assets_and_accepts_relative_recorder(self):
        result = self.capture("a", VHS_BIN="fake tools/selected-vhs")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / "demos/a.gif").read_text(), "fresh gif a")
        self.assertEqual((self.root / "demos/b.gif").read_text(), "prior capture")
        self.assertFalse((self.root / "demos/captures/b.md").exists())

    def test_missing_fresh_output_cannot_be_hidden_by_an_old_recording(self):
        result = self.capture(MISSING_TAPE="b")
        self.assertNotEqual(result.returncode, 0)
        self.assert_prior_captures()

    def test_conversion_and_decode_failures_preserve_the_entire_old_set(self):
        for variable in ("FAIL_CONVERSION", "CORRUPT_OUTPUT"):
            with self.subTest(variable=variable):
                result = self.capture(**{variable: "1"})
                self.assertNotEqual(result.returncode, 0)
                self.assert_prior_captures()

    def test_single_frame_is_not_an_animated_demo(self):
        result = self.capture(SINGLE_FRAME="1")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("multiple frames", result.stderr)
        self.assert_prior_captures()

    def test_ambiguous_build_output_does_not_guess_a_binary(self):
        result = self.capture(AMBIGUOUS_BINARY="1")
        self.assertNotEqual(result.returncode, 0)
        self.assert_prior_captures()

    def test_invalid_selector_fails_before_building(self):
        for arguments in (("../a",), ("missing",), ("a", "b")):
            with self.subTest(arguments=arguments):
                self.assertNotEqual(self.capture(*arguments).returncode, 0)
                self.assertFalse((self.root / "calls.jsonl").exists())
                self.assert_prior_captures()


if __name__ == "__main__":
    unittest.main()
