---
name: rust-packaging
description: >-
  Package and release a Rust project the "sracat-rs / bird_tool_utils" way —
  the conventions Ben Woodcroft (wwood) uses across his Rust crates. Use when
  setting up a freshly `cargo new`ed crate, migrating an existing crate to
  these conventions, or wiring up releases. Covers Cargo.toml metadata and
  profiles, a keep-a-changelog CHANGELOG, the release.py / cargo-release flow,
  cargo-dist prebuilt binaries + shell installer, crates.io + bioconda,
  optional pixi for native/conda deps, a cargo fmt pre-commit hook, and CI +
  release GitHub Actions workflows.
---

# Rust packaging & release (the sracat-rs way)

This skill reproduces how `wwood/sracat-rs` (a binary crate) and
`wwood/bird_tool_utils` (a library crate) are packaged and released, and
extends them with a `cargo fmt` hook and CI. Bundled templates live in
`templates/` next to this file — copy them into the target project and fill in
the placeholders rather than writing these files from scratch.

## First decision: library or binary?

The whole flow forks on this. Determine it before doing anything else (look for
`src/main.rs` + `[[bin]]` vs a pure `src/lib.rs`).

| Aspect          | Library crate (e.g. bird_tool_utils) | Binary crate (e.g. sracat-rs) |
| --------------- | ------------------------------------ | ----------------------------- |
| `Cargo.lock`    | **gitignored**                       | **committed**                 |
| cargo-dist      | not used                             | used (prebuilt binaries)      |
| Release command | `release.py --version X --library-only` | `release.py --version X`   |
| Publish target  | crates.io                            | crates.io + GitHub Release + bioconda |
| GitHub Actions  | CI only                              | CI + `release.yml` (dist)     |

When in doubt or when a crate is both, treat it as a binary crate but still
`cargo publish` the library.

## Setting up a NEW project (`cargo new`)

1. `cargo new CRATE` (add `--lib` for a library). Prefer `edition = "2021"`.
2. **Cargo.toml metadata + profiles.** Merge `templates/Cargo.snippet.toml`
   into the generated `Cargo.toml`: fill `description`, `license`, `repository`
   (+ `homepage`/`documentation`/`readme` for libraries), add `[profile.release]`
   with `lto = true` and `codegen-units = 1`. Add an explicit `[[bin]]` for
   binaries.
3. **LICENSE.** Add a `LICENSE`/`LICENSE.txt` matching the `license` field
   (sracat-rs uses MIT; bird_tool_utils uses GPL-3.0 — follow the project's
   choice).
4. **CHANGELOG.** Copy `templates/CHANGELOG.md`. See the CHANGELOG section below.
5. **.gitignore.** Ensure `/target` is ignored. Library crates also ignore
   `Cargo.lock`; binary crates commit it. If using pixi, ignore `.pixi/*`
   (keep `!.pixi/config.toml`).
6. **cargo fmt hook** — see the dedicated section below (always do this).
7. **CI workflow** — copy `templates/workflows/ci.yml` to
   `.github/workflows/ci.yml`.
8. **Release tooling:**
   - Copy `templates/release.py` to `scripts/release.py` (chmod +x).
   - Optionally copy `templates/release.toml` if you prefer `cargo release`.
   - Binary crates: initialise cargo-dist — see the cargo-dist section.
9. `git init` if needed, commit, push, and confirm CI is green.

## MIGRATING an existing project

Do the same as a new project, but additively and carefully:

1. Classify it (library vs binary) and fix `Cargo.lock` tracking accordingly
   (add to / remove from git + `.gitignore`).
2. Backfill `Cargo.toml` metadata and the `[profile.release]` / `[profile.dist]`
   / `[workspace.metadata.dist]` blocks that are missing (don't clobber existing
   dependency or feature config).
3. If there's no CHANGELOG, create one and reconstruct recent history from
   `git tag` / release notes into `## Version X.Y.Z` sections, newest first,
   with a `## Unreleased` at the top.
4. Add the cargo fmt hook + `ci.yml`. Run `cargo fmt --all` once as its own
   commit ("cargo fmt") so the formatting churn doesn't pollute later diffs,
   then fix any `cargo clippy -- -D warnings` findings.
5. Add `scripts/release.py`; for binaries run `dist init`.
6. Verify: `cargo fmt --all -- --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test`, and for binaries `dist plan`.

## CHANGELOG conventions

Keep-a-changelog style, newest first. A `## Unreleased` section sits at the top;
under it and each release use `### Added` / `### Fixed` / `### Changed`
subsections. Release headings are `## Version X.Y.Z`. During a release the
tooling moves the `Unreleased` bullets into a new `## Version X.Y.Z` section —
so **write user-facing notes under `## Unreleased` as you go**, and both
`release.py` and `release.toml` will roll them forward for you.

## cargo fmt hook (do this on every project)

Two layers — a local hook for fast feedback, and a CI job that cannot be
bypassed.

1. **Local pre-commit hook.** Either:
   - **Plain git hook (no dependencies):** copy `templates/pre-commit` to
     `scripts/pre-commit`, then install it in each clone:
     `ln -sf ../../scripts/pre-commit .git/hooks/pre-commit`. It runs
     `cargo fmt --all -- --check` and blocks the commit on unformatted code.
     Because git hooks aren't committed, document that install line in the
     README/CONTRIBUTING.
   - **pre-commit framework:** copy `templates/.pre-commit-config.yaml` to the
     repo root and run `pre-commit install`. Prefer this only if the project
     already uses pre-commit; it also wires up clippy.
2. **CI enforcement.** `templates/workflows/ci.yml` includes a dedicated
   `rustfmt` job (`cargo fmt --all -- --check`) — this is the real guard, since
   a local hook can be skipped with `--no-verify`.
3. Rely on default rustfmt style. Only add a `rustfmt.toml` if the project
   genuinely needs overrides; keep it minimal and commit it.

## GitHub Actions

1. **CI (`ci.yml`) — every crate.** Copy `templates/workflows/ci.yml` to
   `.github/workflows/ci.yml`. It runs on push to `main` and all PRs:
   - `rustfmt` job: `cargo fmt --all -- --check`
   - `clippy + test` job: `cargo clippy --all-targets --all-features -- -D warnings`
     then `cargo test --all`, with `Swatinem/rust-cache` for speed.
   For crates with native/conda deps, swap the toolchain setup for the pixi
   block documented at the bottom of that template.
2. **Release (`release.yml`) — binary crates only.** This is **autogenerated by
   cargo-dist** — do not hand-write or hand-edit it. Run `dist init` (see next
   section); it writes `.github/workflows/release.yml`. It triggers on tags
   matching `**[0-9]+.[0-9]+.[0-9]+*` (and on PRs, for validation), builds the
   configured targets, and creates a GitHub Release with the artifacts + shell
   installer. Re-run `dist generate` after changing dist config so the workflow
   stays in sync.
3. **build-setup (native deps).** If the crate links native/conda libraries,
   `cargo build` in the release runner won't find them. Point
   `github-build-setup` in Cargo.toml at
   `templates/workflows/build-setup.yml` (place it under
   `.github/workflows/build-setup/`) to provision them with pixi before each
   build. See how sracat-rs static-links ncbi-vdb.

## cargo-dist (binary crates)

Prebuilt binaries + a `curl | sh` installer are produced by
[cargo-dist](https://opensource.axo.dev/cargo-dist/):

1. Install the pinned `dist` (match `cargo-dist-version` in Cargo.toml, e.g.
   0.31.0).
2. `dist init` — writes `[workspace.metadata.dist]` and
   `.github/workflows/release.yml`. Then edit the metadata toward the template
   values: `installers = ["shell"]`, `install-updater = false`,
   `install-path = "CARGO_HOME"`, and a **deliberate** `targets` list (default
   is broad; sracat-rs restricts to `x86_64-unknown-linux-gnu` because its
   conda ncbi-vdb dependency only exists for glibc x86_64 — only widen targets
   once a matching build env/runner exists).
3. `dist plan` locally to sanity-check before tagging.

## Release process

Both crates use `scripts/release.py` (bundled as `templates/release.py`). It:
validates the version, refuses a dirty tree / existing tag, confirms + rolls the
CHANGELOG `Unreleased` section into `## Version X.Y.Z`, bumps the Cargo.toml
version, `cargo update`, `cargo test`, (binaries) `dist plan`, commits, tags
`vX.Y.Z`, `cargo publish`, and pushes with tags.

- **Library:** `python scripts/release.py --version X.Y.Z --library-only`
  (skips cargo-dist; publishes the crate to crates.io).
- **Binary:** `python scripts/release.py --version X.Y.Z` — the pushed tag
  triggers `release.yml`, which builds and attaches the binaries + installer to
  the GitHub Release. Then update the bioconda recipe and open a PR against
  `bioconda-recipes`.
- `cargo release X.Y.Z` (via `release.toml`) is an equivalent alternative if you
  prefer it to the Python script; don't run both.

Prerelease versions (`X.Y.Z-beta.1`) are supported by the version validator and
are marked as prereleases by cargo-dist automatically.

## pixi (optional — native / conda dependencies)

Only for crates that depend on non-Rust libraries (like sracat-rs → ncbi-vdb).
A `pixi.toml` declares conda channels (`conda-forge`, `bioconda`), the native
deps, the Rust toolchain, and `[tasks]` (`build`, `test`, ...). `build.rs` reads
`CONDA_PREFIX` from the pixi env to find headers/libs. If you use pixi, add a
`.gitattributes` line so the lockfile doesn't cause merge noise:
`pixi.lock merge=binary linguist-language=YAML linguist-generated=true -diff`,
and gitignore `.pixi/*` (keeping `!.pixi/config.toml`). Pure-Rust crates ignore
this whole section.

## Verification checklist

Before considering a project done:
- [ ] `cargo fmt --all -- --check` clean; pre-commit hook installed/documented.
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` clean.
- [ ] `cargo test` (or `pixi run test`) passes.
- [ ] `Cargo.toml` has description, license, repository, and release profile.
- [ ] `Cargo.lock` tracked (binary) or gitignored (library).
- [ ] CHANGELOG has an `## Unreleased` section.
- [ ] `.github/workflows/ci.yml` present and green.
- [ ] Binary crates: `dist plan` succeeds and `release.yml` exists.
