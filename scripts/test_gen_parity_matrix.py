#!/usr/bin/env python3
"""Tests for gen-parity-matrix.py (run with `python3 scripts/test_gen_parity_matrix.py`)."""
import importlib.util
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("gen", Path(__file__).with_name("gen-parity-matrix.py"))
gen = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gen)

CASES = [("a", "ValueCase"), ("b", "MappingCase"), ("c", "TagCase")]


def write(text):
    f = tempfile.NamedTemporaryFile("w", suffix=".gram", delete=False)
    f.write(text)
    f.close()
    return Path(f.name)


class ParityMatrix(unittest.TestCase):
    def test_read_results_reads_status_and_reason(self):
        p = write(
            '(a:Result {language: x, status: passed})\n'
            '(b:Result {language: x, status: skipped, reason: "needs \\"BigInt\\""})\n'
            '(c:Result {language: x, status: not-applicable})\n'
        )
        self.assertEqual(
            gen.read_results(p),
            {"a": ("passed", None), "b": ("skipped", 'needs "BigInt"'), "c": ("not-applicable", None)},
        )

    def test_a_missing_result_is_marked_missing(self):
        cells = gen.build(CASES, {"x": {"a": ("passed", None)}})
        self.assertEqual(cells[("b", "x")][0], "missing")

    def test_render_summarizes_and_lists_gaps(self):
        results = {
            "x": {"a": ("passed", None), "b": ("skipped", "later"), "c": ("failed", None)},
            "y": {"a": ("passed", None), "b": ("not-applicable", None), "c": ("passed", None)},
        }
        text = gen.render(CASES, results, gen.build(CASES, results))
        self.assertIn("| x | 1 | 1 | 1 | 0 | 0 |", text)
        self.assertIn("| y | 2 | 0 | 0 | 1 | 0 |", text)
        self.assertIn("| b | x | ⏭️ skipped | later |", text)
        self.assertIn("| c | x | ❌ failed |", text)
        self.assertIn("| MappingCase | 1 | 0/1 | 0/0 |", text)

    def test_a_clean_run_has_no_gaps(self):
        results = {"x": {c: ("passed", None) for c, _ in CASES}}
        text = gen.render(CASES, results, gen.build(CASES, results))
        self.assertIn("None: every case passes", text)

    def test_exit_status_fails_on_failed_or_missing_but_not_on_skipped(self):
        ids = [c for c, _ in gen.read_cases(gen.ROOT / "conformance")]

        def run(overrides, drop=()):
            lines = [
                f"({c}:Result {{language: x, status: {overrides.get(c, 'passed')}}})"
                for c in ids
                if c not in drop
            ]
            p = write("\n".join(lines) + "\n")
            out = Path(tempfile.mkdtemp()) / "m.md"
            return gen.main([f"x={p}", "--out", str(out)])

        self.assertEqual(run({}), 0)
        self.assertEqual(run({ids[0]: "skipped"}), 0)
        self.assertEqual(run({ids[0]: "not-applicable"}), 0)
        self.assertEqual(run({ids[0]: "failed"}), 1)
        self.assertEqual(run({}, drop=[ids[0]]), 1)


if __name__ == "__main__":
    unittest.main()
