# Releasing

How to publish a package. The decisions behind this are in [ADR-007](design/adr/ADR-007-release-process-and-versions.md).

Each package is released by its own tag: `rust-vX.Y.Z`, `typescript-vX.Y.Z`, `python-vX.Y.Z`, for Go `go/vX.Y.Z` (the form Go requires for a module in a subdirectory), and for Swift the plain `vX.Y.Z` (SwiftPM only sees plain semantic-version tags). The packages and the
corpus (`conformance/VERSION`) share a `major.minor`; a fix to one package is a patch of that package only.

## One-time setup (the project owner)

Do these once, before the first release. The workflow cannot do them.

1. **GitHub.** In the repository settings, create an environment named `release` and add yourself (or a team) as
   a required reviewer.
2. **crates.io.** Create the crate name by the first publish (below). Make the GitHub team of the organization an
   owner of the crate, then add a trusted publisher: repository `gram-data/gram-value`, workflow `release.yml`,
   environment `release`.
3. **npm.** Create the organization `gram-data` (the scope `@gram-data` exists for the grammar package). After the first
   publish, add a trusted publisher to `@gram-data/value`: repository `gram-data/gram-value`, workflow `release.yml`,
   environment `release`.
4. **PyPI.** Register a pending trusted publisher for the project name `gram-value`: repository
   `gram-data/gram-value`, workflow `release.yml`, environment `release`. No token is needed.
5. **A new crate or npm package.** crates.io and npm can only configure trusted publishing for a package that already
   exists, and the workflow uses no stored token. Publish the very first version of a new package by hand
   (`cargo publish` from `rust/`, or `npm publish --access public` from `typescript/`) with your own credentials,
   then configure its trusted publisher. `gram-value` (crates.io) and `@gram-data/value` (npm) are already past this
   step. The organization has secrets named `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`; the release workflow does not
   read them.

## Releasing a package

1. Set the version in the package's manifest (`rust/Cargo.toml`, `typescript/package.json`,
   `python/pyproject.toml`, and `go/VERSION` and `swift/VERSION` for Go and Swift, which take their version from the tag). For a new minor, set it in all five and in `conformance/VERSION`.
2. Check: `python3 scripts/check-release.py rust-v0.2.0` (use your tag).
3. Merge to `main` and wait for CI to pass.
4. Tag the merge commit and push the tag:
   ```sh
   git tag rust-v0.2.0 && git push origin rust-v0.2.0
   ```
5. The `Release` workflow verifies the tag, then waits. Approve the `publish-*` job in the Actions tab.
6. The job publishes, polls the registry until the version is visible, and creates a GitHub release.

## When something fails

- npm can take several minutes to show a new version (the first publish took about five), so the verification waits up
  to 15 minutes. If it still times out, check the package page before re-running.
- Re-run the failed job from the Actions tab. It skips publishing a version that is already on the registry and
  completes what is missing.
- A published version is never replaced. If it is defective, yank it (crates.io, PyPI) or deprecate it (npm) and
  release a new patch.
- The workflow's checks fail before anything is published if the tag does not match the manifest, the commit is not on
  `main`, or CI did not pass for the commit.

## Go

Go modules have no registry account: the module `github.com/gram-data/gram-value/go` is published by pushing the
tag `go/vX.Y.Z`. The release workflow still verifies the tag, waits for approval, asks the Go module proxy
(`proxy.golang.org`) for the version, checks that it is served, and creates the GitHub release. Nothing needs to be
set up in advance. A published Go version cannot be replaced either (the proxy caches it); retract a defective one
with a `retract` directive in `go/go.mod` and release a new patch.

## Swift

Swift packages have no registry account either: the package `https://github.com/gram-data/gram-value` is published by
pushing the plain tag `vX.Y.Z`. The release workflow verifies the tag, waits for approval, checks that a scratch
package resolves that version as a dependency, and creates the GitHub release. A published tag is never moved or
reused; fix a defective version with a new patch. The tag `vX.Y.Z` always means the Swift package, so use it for
nothing else.
