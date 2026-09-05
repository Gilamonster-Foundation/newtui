import pathlib
import re
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
PYTHON_BLOCK = re.compile(r"```python\n(.*?)```", re.DOTALL)


class DocumentationExamples(unittest.TestCase):
    def test_every_python_example_runs(self):
        paths = [ROOT / "examples/python/README.md", ROOT / "docs/CATALOG.md"]
        checked = 0

        for path in paths:
            text = path.read_text(encoding="utf-8")
            for index, block in enumerate(PYTHON_BLOCK.findall(text), start=1):
                with self.subTest(path=path.relative_to(ROOT), block=index):
                    exec(compile(block, f"{path} block {index}", "exec"), {})
                checked += 1

        self.assertGreater(checked, 0, "the documentation scan executed nothing")


if __name__ == "__main__":
    unittest.main()
