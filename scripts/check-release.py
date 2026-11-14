#!/usr/bin/env python3
"""Check the versions of the packages and of a release tag (ADR-007).

    scripts/check-release.py                    check that every package shares the corpus's major.minor
    scripts/check-release.py rust-v0.1.0        also check that a release tag matches its package's version

Prints the package and version of a valid tag as `package=<name>` and `version=<x.y.z>` lines (for
GitHub Actions outputs) and exits with 1 and a message on a problem.
"""
import json
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PACKAGES = ("rust", "typescript", "python")
TAG = re.compile(r"^(?P<package>rust|typescript|python)-v(?P<version>\d+\.\d+\.\d+)$")
SEMVER = re.compile(r"^\d+\.\d+\.\d+$")


def corpus_version(root=ROOT):
    return (root / "conformance" / "VERSION").read_text().strip()


def package_version(package, root=ROOT):
    if package == "rust":
        return tomllib.loads((root / "rust" / "Cargo.toml").read_text())["package"]["version"]
    if package == "typescript":
        return json.loads((root / "typescript" / "package.json").read_text())["version"]
    return tomllib.loads((root / "python" / "pyproject.toml").read_text())["project"]["version"]


def major_minor(version):
    return ".".join(version.split(".")[:2])


def check(tag=None, root=ROOT):
    """Return a list of problems (empty when all is well) and the (package, version) of the tag."""
    problems = []
    corpus = corpus_version(root)
    if not re.fullmatch(r"\d+\.\d+", corpus):
        problems.append(f"conformance/VERSION must be major.minor, found {corpus!r}")
    for package in PACKAGES:
        v = package_version(package, root)
        if not SEMVER.match(v):
            problems.append(f"{package}: version {v!r} is not major.minor.patch")
        elif major_minor(v) != corpus:
            problems.append(f"{package}: version {v} does not share major.minor {corpus} with the corpus")
    parsed = None
    if tag is not None:
        m = TAG.match(tag)
        if not m:
            problems.append(f"tag {tag!r} is not <rust|typescript|python>-vMAJOR.MINOR.PATCH")
        else:
            package, version = m.group("package"), m.group("version")
            parsed = (package, version)
            if package_version(package, root) != version:
                problems.append(
                    f"tag {tag} does not match the {package} package version {package_version(package, root)}"
                )
    return problems, parsed


def main(argv):
    problems, parsed = check(argv[0] if argv else None)
    for p in problems:
        print(f"error: {p}", file=sys.stderr)
    if problems:
        return 1
    if parsed:
        print(f"package={parsed[0]}")
        print(f"version={parsed[1]}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
