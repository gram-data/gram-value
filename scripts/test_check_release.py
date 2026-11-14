#!/usr/bin/env python3
"""Tests for check-release.py (run with `python3 scripts/test_check_release.py`)."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location("chk", Path(__file__).with_name("check-release.py"))
chk = importlib.util.module_from_spec(spec)
spec.loader.exec_module(chk)


def tree(corpus="0.2", rust="0.2.0", ts="0.2.0", py="0.2.1", go="0.2.0", swift="0.2.0"):
    root = Path(tempfile.mkdtemp())
    for d in ("conformance", "rust", "typescript", "python", "go", "swift"):
        (root / d).mkdir()
    (root / "conformance" / "VERSION").write_text(corpus + "\n")
    (root / "rust" / "Cargo.toml").write_text(f'[package]\nname = "x"\nversion = "{rust}"\n')
    (root / "typescript" / "package.json").write_text(json.dumps({"name": "x", "version": ts}))
    (root / "python" / "pyproject.toml").write_text(f'[project]\nname = "x"\nversion = "{py}"\n')
    (root / "go" / "VERSION").write_text(go + "\n")
    (root / "swift" / "VERSION").write_text(swift + "\n")
    return root


class CheckRelease(unittest.TestCase):
    def test_a_shared_major_minor_with_a_different_patch_is_fine(self):
        problems, parsed = chk.check("python-v0.2.1", tree())
        self.assertEqual(problems, [])
        self.assertEqual(parsed, ("python", "0.2.1"))

    def test_go_tags_use_the_subdirectory_form(self):
        problems, parsed = chk.check("go/v0.2.0", tree())
        self.assertEqual(problems, [])
        self.assertEqual(parsed, ("go", "0.2.0"))
        problems, parsed = chk.check("go-v0.2.0", tree())
        self.assertIsNone(parsed)
        self.assertTrue(problems)

    def test_the_plain_v_tag_is_the_swift_release(self):
        problems, parsed = chk.check("v0.2.0", tree())
        self.assertEqual(problems, [])
        self.assertEqual(parsed, ("swift", "0.2.0"))
        problems, _ = chk.check("v0.2.3", tree())
        self.assertTrue(any("does not match" in p for p in problems))

    def test_a_package_on_another_minor_is_reported(self):
        problems, _ = chk.check(None, tree(ts="0.3.0"))
        self.assertEqual(len(problems), 1)
        self.assertIn("typescript", problems[0])

    def test_a_tag_must_match_the_package_version(self):
        problems, _ = chk.check("rust-v0.2.5", tree())
        self.assertTrue(any("does not match" in p for p in problems))

    def test_a_malformed_tag_is_reported(self):
        for tag in ("v0.2", "swift-v0.2.0", "rust-0.2.0", "rust-v0.2", "go/0.2.0", "vv0.2.0"):
            problems, parsed = chk.check(tag, tree())
            self.assertIsNone(parsed, tag)
            self.assertTrue(problems, tag)

    def test_the_corpus_version_is_major_minor(self):
        problems, _ = chk.check(None, tree(corpus="0.2.0"))
        self.assertTrue(any("major.minor" in p for p in problems))

    def test_the_repository_is_consistent(self):
        problems, _ = chk.check()
        self.assertEqual(problems, [])


if __name__ == "__main__":
    unittest.main()
