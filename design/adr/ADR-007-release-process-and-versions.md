---
id: ADR-007
title: Releasing the Packages: Versions, Names, Trusted Publishing and an Approval Gate
status: accepted
rfcs: [RFC-003]
created: 2026-11-14
---

# ADR-007: Releasing the Packages: Versions, Names, Trusted Publishing and an Approval Gate

## 1. Context

`gram-value` is published as a package in each language's registry: crates.io for Rust, npm for TypeScript and PyPI for Python now, with Go modules, Swift packages and Hackage to follow. The packages live in one repository and pass one conformance corpus, so a user who combines two languages needs to know which versions agree. Registries differ: a published version cannot be replaced (crates.io, npm and PyPI never reuse a version number), a package can only be removed or "yanked" afterwards, and each registry has its own login. Go and Swift read versions from repository tags, which in a repository with several packages means one tag prefix per package.

Releasing by hand from a laptop leaves the knowledge with one person and invites a registry left a version behind the others. Long-lived API tokens in a repository need rotating and are a target. crates.io, npm and PyPI each support trusted publishing: the registry accepts a short-lived credential that GitHub Actions issues for one named repository, workflow and environment, so no token is stored.

## 2. Decision

### 2.1 Versions: shared major.minor, per-package patch

The corpus, in the file `conformance/VERSION`, and every package share a `major.minor` version. It changes whenever behavior or the corpus changes in a way that affects what a package must do (a new kind, a new known tag, a changed mapping, a new corpus case a package must pass). Two packages with the same `major.minor` agree on the behavior the corpus describes. A fix confined to one package is a patch of that package only (Python `0.2.1` while Rust stays `0.2.0`). Before 1.0, a minor version may change behavior; after 1.0, a major version is a breaking change (adding a kind is one, ADR-002 2.4). `scripts/check-release.py` fails when a package's version has a different `major.minor` from the corpus, and continuous integration runs it on every change.

### 2.2 Package names and ownership

| Registry | Name | Owner |
|---|---|---|
| crates.io | `gram-value` | the GitHub organization `gram-data` (a team owner) |
| npm | `@gram-data/value` | the npm organization `gram-data` |
| PyPI | `gram-value` | the PyPI organization or maintainers named by the project owner |
| Go | `github.com/gram-data/gram-value/go` | the repository |
| Swift | `https://github.com/gram-data/gram-value` (product `GramValue`) | the repository |
| Hackage (later) | `gram-value` | decided when Haskell joins |

The npm package is scoped, like the organization's grammar package `@gram-data/tree-sitter-gram`, so the name belongs to the organization. crates.io and PyPI have no scopes, and the name `gram-value` is registered once for each.

### 2.3 Tags start a release

A release of one package is a tag `<package>-v<major>.<minor>.<patch>` on a commit of `main`: `rust-v0.2.0`, `typescript-v0.2.0`, `python-v0.2.1`. The Go package uses the tag form Go requires for a module in a subdirectory (`go/v0.2.0`). The tag's version must equal the package's version in its manifest (`Cargo.toml`, `package.json`, `pyproject.toml`).

### 2.4 A release workflow verifies, waits for approval, publishes and verifies

`.github/workflows/release.yml` runs on a package tag and does these in order:

1. **Verify:** the tag matches its package's version and the packages share the corpus's `major.minor` (`scripts/check-release.py`); the tagged commit is on `main`; every continuous integration job (`rust`, `typescript`, `python`, `corpus`) succeeded for that commit.
2. **Wait for approval:** the publish job belongs to the protected GitHub environment `release`, which requires a reviewer's approval before it starts.
3. **Publish:** to the registry of that package, unless that version is already published there.
4. **Verify from the registry:** poll the registry until the version is visible, and fail otherwise.
5. **Create a GitHub release** for the tag with generated notes, unless one exists.

Each package is released by its own tag and its own job, so a failure in one registry does not stop the others. A version that was published is never replaced.

### 2.5 Trusted publishing, with a token only for a first release

The publish jobs authenticate with GitHub's OIDC identity (`id-token: write`): `rust-lang/crates-io-auth-action` for crates.io, `pypa/gh-action-pypi-publish` for PyPI (a pending publisher can be registered before the first upload), and npm's own trusted publishing with provenance. crates.io and npm can only configure trusted publishing for a package that already exists, so the first release of those two uses a temporary token stored as the repository secret `CARGO_REGISTRY_TOKEN` or `NPM_TOKEN`; the project owner deletes the secret after the first publish and configures trusted publishing in the registry. The workflow uses the token only when its secret exists. Creating the registry accounts, organizations and trusted-publisher entries is the project owner's task, and `RELEASING.md` lists each step.

### 2.6 Recovery

A failed publish job is re-run from the Actions tab. The job first checks whether the version is already on the registry and skips publishing if so, so a re-run completes whatever is missing (the verification and the GitHub release). A published version that turns out defective is yanked (crates.io, PyPI) or deprecated (npm) and fixed in a new patch version; it is never overwritten. If one package's release fails for a cause that needs a code change, the fix is a new patch of that package only (2.1).

### 2.7 Go (added 2026-11-14)

The Go module is `github.com/gram-data/gram-value/go`, in the directory `go/`, released by the tag `go/vX.Y.Z`. A Go module takes its version from the tag, not from a file, so `go/VERSION` records it for `scripts/check-release.py`, which checks it against the tag and the shared `major.minor`. There is no registry account, no token and no trusted-publisher setup. The release workflow verifies the tag, waits for the same approval, asks the Go module proxy (`proxy.golang.org`) for the version, checks that it is served, and creates the GitHub release. A defective version is retracted with a `retract` directive in `go/go.mod` and fixed by a new patch.

### 2.8 Swift (added 2026-11-14)

SwiftPM reads the manifest at the root of a repository, as tree-sitter-gram's Swift binding does, so `Package.swift` sits at the root of this repository and points at `swift/Sources/GramValue`. The library has no dependencies. SwiftPM downloads every dependency a manifest declares, even ones only tests use, so the conformance runner and unit tests are a second package, `swift/Tests/Package.swift`, which depends on the root by path and on tree-sitter-gram and SwiftTreeSitter. SwiftPM only recognizes plain semantic-version tags, and a tag always names the whole repository, so the Swift release is the unprefixed tag `vX.Y.Z` (the other packages keep their prefixed tags, so there is no clash). `swift/VERSION` records the version for `scripts/check-release.py`. There is no registry account, token or trusted-publisher setup. The release workflow verifies the tag, waits for the same approval, checks that a scratch package resolves the tagged version as a dependency, and creates the GitHub release. A defective version is not replaced; it is fixed by a new patch.

### 2.9 The first releases and the removal of the token fallbacks (added 2026-11-14)

The first release of crates.io and npm needed a token, as 2.5 says: the first npm publish (`0.1.0`) used a token created for the purpose, and the first crates.io publish used the organization's `CARGO_REGISTRY_TOKEN`. Trusted publishing was then configured for both packages, and the second npm release (`0.1.1`) was recorded by the registry as published by `GitHub Actions` through the trusted publisher. The workflow's token fallbacks were then removed, so no release job references a stored token: a misconfigured trusted publisher fails the job instead of quietly using a long-lived credential. Two organization-level secrets, `CARGO_REGISTRY_TOKEN` and `NPM_TOKEN`, are visible to this repository (`gh secret list --repo` lists only repository secrets, so they do not show there; `gh api repos/<owner>/<repo>/actions/organization-secrets` does), and the workflow no longer reads them. The first release of any new crate or npm package is published by hand by the project owner (`cargo publish`, `npm publish --access public`) before its trusted publisher is configured.

## 3. Alternatives Considered

3.1. **One number for every package.** Rejected: a fix to one package would republish identical copies of the others, and Go and Swift need per-package tags anyway.

3.2. **Independent versions per package.** Rejected: a user combining languages could not tell which versions agree, the problem RFC-003 begins with.

3.3. **Long-lived API tokens as repository secrets for every release.** Rejected: they sit in the repository, need rotating, and a leaked one publishes under the project's name. They remain only for the first release of crates.io and npm, where the registries require them.

3.4. **Publish on every tag push, with no approval.** Rejected: a mistaken tag would publish a version that cannot be unpublished; the approval costs one click.

3.5. **A manually run workflow that names the package and version.** Rejected: the tag is then not the record of the decision, and nothing ties the published artifact to a commit a person chose.

3.6. **An unscoped npm name `gram-value`.** Rejected: an unscoped name belongs to whichever account publishes first, and a lookalike is easy to register; the scope makes the organization the owner.

3.7. **One tag that releases all packages.** Rejected: it contradicts per-package patches, and one registry's outage would block the others.

## 4. Consequences

- A release is: bump the version in the manifest (and `conformance/VERSION` for a new minor), merge to `main`, wait for green continuous integration, push the tag, approve the job. `RELEASING.md` holds the checklist.
- The first release needs the owner to create the registry organizations, reserve the names, set the temporary tokens, create the `release` environment with a reviewer, and register the trusted publishers afterwards. Until then the workflow's publish jobs fail at their authentication step, and nothing is published by accident.
- crates.io and npm trusted publishing could not be exercised before the first release; the workflow's authentication steps for them are untested until then.
- The `corpus` job is a dependency of every release, since the verification requires it to have passed.
- A user can tell whether two packages agree by their `major.minor`; a patch difference says only that one package was fixed.
- The plain `vX.Y.Z` tag is reserved for the Swift release; a different package must not use it.
- Version bumps for a new minor touch four files (the manifests, the `go/VERSION` and `swift/VERSION` files and `conformance/VERSION`); `check-release.py` catches a missed one.
- Follow-up work: `RELEASING.md`; a changelog policy; the Go, Swift and Haskell tag and name decisions; whether to attach the parity matrix to each GitHub release.

## 5. Related

- RFC-003 (resolves questions 7.6, 7.7 and 7.8).
- ADR-001 (the corpus whose version the packages share); ADR-002 2.4 (what a breaking change is).
