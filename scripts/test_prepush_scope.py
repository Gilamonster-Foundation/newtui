"""The pre-push scope must skip only pushes that send no code, and fail closed.

Each case builds a throwaway repository and feeds the real script the same
stdin protocol git gives the hook, so the decision is checked end to end.
"""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("prepush-scope.sh").resolve()
ZERO = "0" * 40


class ScopeTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.git("init", "-q", "-b", "main")
        self.git("config", "user.email", "t@example.invalid")
        self.git("config", "user.name", "t")
        self.write("tests/catalog.rs", 'include_str!("../docs/CATALOG.md");\n')
        self.write("docs/CATALOG.md", "catalog\n")
        self.write("docs/NOTES.md", "notes\n")
        self.write("src/lib.rs", "// lib\n")
        self.base = self.commit("base")
        self.git("branch", "origin/main")  # stands in for the remote base

    def tearDown(self):
        self.tmp.cleanup()

    def git(self, *args):
        return subprocess.run(
            ["git", *args], cwd=self.root, check=True, text=True, capture_output=True
        ).stdout.strip()

    def write(self, path, text):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text)

    def commit(self, message):
        self.git("add", "-A")
        self.git("commit", "-qm", message)
        return self.git("rev-parse", "HEAD")

    def scope(self, *lines):
        stdin = "".join(f"{' '.join(line)}\n" for line in lines)
        env = {**os.environ, "PREPUSH_BASE": "origin/main"}
        out = subprocess.run(
            ["bash", str(SCRIPT)], cwd=self.root, input=stdin,
            text=True, capture_output=True, env=env, check=True,
        ).stdout.strip()
        return out.split(":", 1)[0], out

    def push(self, remote_sha=ZERO):
        head = self.git("rev-parse", "HEAD")
        return ("refs/heads/topic", head, "refs/heads/topic", remote_sha)

    def test_unread_markdown_skips(self):
        self.write("docs/NOTES.md", "changed\n")
        self.write("docs/NEW.md", "new\n")
        self.commit("docs")
        self.assertEqual(self.scope(self.push())[0], "skip")

    def test_markdown_included_by_rust_runs_the_gate(self):
        self.write("docs/CATALOG.md", "changed\n")
        self.commit("catalog")
        verdict, out = self.scope(self.push())
        self.assertEqual(verdict, "full", out)
        self.assertIn("read by code", out)

    def test_any_code_runs_the_gate(self):
        self.write("docs/NOTES.md", "changed\n")
        self.write("src/lib.rs", "// changed\n")
        self.commit("mixed")
        self.assertEqual(self.scope(self.push())[0], "full")

    def test_branch_delete_skips(self):
        self.assertEqual(self.scope(("(delete)", ZERO, "refs/heads/topic", self.base))[0], "skip")

    def test_update_diffs_against_the_remote_sha(self):
        self.write("docs/NOTES.md", "one\n")
        pushed = self.commit("first")
        self.write("src/lib.rs", "// second\n")
        self.commit("second")
        self.assertEqual(self.scope(self.push(remote_sha=pushed))[0], "full")

    def test_unknown_remote_sha_fails_closed(self):
        self.write("docs/NOTES.md", "changed\n")
        self.commit("docs")
        self.assertEqual(self.scope(self.push(remote_sha="1" * 40))[0], "full")

    def test_new_branch_without_a_base_fails_closed(self):
        self.write("docs/NOTES.md", "changed\n")
        self.commit("docs")
        self.git("branch", "-D", "origin/main")
        self.assertEqual(self.scope(self.push())[0], "full")

    def test_one_code_push_among_docs_pushes_runs_the_gate(self):
        self.write("docs/NOTES.md", "changed\n")
        self.commit("docs")
        docs = self.push()
        self.write("src/lib.rs", "// changed\n")
        self.commit("code")
        self.assertEqual(self.scope(docs, self.push())[0], "full")


if __name__ == "__main__":
    unittest.main()
