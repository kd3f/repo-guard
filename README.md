# Repo Guard

![License](https://img.shields.io/badge/license-Apache--2.0-blue)
![Rust](https://img.shields.io/badge/rust-1.92.0+-orange)

**Repo Guard is a Git and Git LFS file-safety policy engine — not a hook manager.**

It complements [pre-commit](https://pre-commit.com/), [Husky](https://typicode.github.io/husky/), [Lefthook](https://github.com/evilmartians/lefthook), custom Git hooks, and CI workflows. Repo Guard focuses on one job: catching oversized regular Git blobs, Git LFS misconfiguration, staged-index policy violations, and risky outgoing history **before** they reach a remote.

**v0.2.0** is an early public release. Behavior is validated on Windows, macOS, and Linux in CI, but you should still evaluate fit for your team.

## The problem

Git stores file content as blobs. Large binaries committed as normal Git objects slow clones, blow past host limits, and are painful to fix after push. Git LFS helps, but only when files are actually tracked as LFS pointers — not when someone `git add`s a 200 MiB zip as a regular blob.

Repo Guard inspects what Git will commit or push, applies your thresholds, and prints **exact repair commands** when something is wrong.

### Example: blocked staged file

A **50 MiB regular Git blob** (for example `assets/model.pack` at exactly 52 428 800 bytes) fails the LFS threshold — not only files above 95 MiB.

```text
$ repo-guard check
repo-guard check
Status: failed (exit 2)

Checks:
  [ERROR] blob.lfs_required: assets/model.pack is a 52428800 byte regular Git blob at or above the 50 MiB LFS threshold
         Files at or above 50 MiB must be stored through Git LFS.
  [INFO] repair.commands: Copy and paste these commands to repair the staged index:
  [INFO] repair.command: git lfs track --filename "assets/model.pack"
  [INFO] repair.command: git add .gitattributes
  [INFO] repair.command: git add --renormalize -- "assets/model.pack"
  [INFO] repair.command: repo-guard check
```

Copy the printed repair steps, re-run `repo-guard check`, then commit.

## Quick start

From a Git repository you want to protect:

```bash
repo-guard doctor          # Git, Git LFS, hooks, config sanity
repo-guard detect          # Who owns hooks? standalone vs integrate?
repo-guard init            # Preview setup (no writes)
repo-guard init --apply    # writes config; adds .githooks only in standalone mode
repo-guard check           # Validate the staged index before commit
```

`init` and `install` stay read-only until you pass `--apply`.

## Choose your integration path

| Situation | What to do |
|---|---|
| No other hook manager | `repo-guard detect` → if standalone is recommended, `init --apply` / `install --apply` |
| pre-commit, Husky, or Lefthook | `repo-guard detect` → `repo-guard integrate pre-commit --print` (or husky/lefthook) → add snippets manually |
| Team enforcement | CI: `repo-guard history check --range <base>..<head>` (see [CI enforcement](#ci-enforcement)) |
| pre-commit framework consumers | Use hook id **`repo-guard-check`** from `.pre-commit-hooks.yaml` after publishing this repo |

Repo Guard **does not** take over `core.hooksPath` when another manager is detected, and **does not** support external-manager pre-push integration in v0.2.0. Use standalone `.githooks` or CI `history check` for push-time policy.

See [docs/compatibility-matrix.md](docs/compatibility-matrix.md) for evidence levels.

## Installation

Repo Guard is distributed as **source** today. Build or install with Cargo; there are no crates.io packages or pre-built release binaries yet.

**Requirements:** [Git](https://git-scm.com/), [Git LFS](https://git-lfs.com/) (for LFS checks), Rust **1.92.0+** (see `rust-toolchain.toml`).

```bash
git clone https://github.com/kd3f/repo-guard.git
cd repo-guard
cargo install --path . --locked
repo-guard --version
```

`cargo install --path . --locked` places `repo-guard` in Cargo's bin directory (typically `%USERPROFILE%\.cargo\bin` on Windows or `$HOME/.cargo/bin` on Unix). Ensure that directory is on your `PATH`, then restart terminals or GUI Git clients.

To build without installing:

```bash
cargo build --release
```

The binary is then at `target/release/repo-guard` (or `target\release\repo-guard.exe` on Windows). That path is local to the build tree unless you copy the binary elsewhere or use `cargo install`.

## Core commands

| Command | Purpose |
|---|---|
| `repo-guard doctor` | Validate Git, Git LFS, hooks, remotes, and configuration |
| `repo-guard detect` | Read-only hook-manager detection and mode recommendation |
| `repo-guard init [--apply]` | Create `.repo-guard.toml` and shared hooks (standalone only) |
| `repo-guard install [--apply]` | Activate `.githooks` and `git lfs install --local` |
| `repo-guard check` | Validate **staged** index content against policy |
| `repo-guard scan` | Scan working tree for large files (respects `.gitignore`) |
| `repo-guard lfs check` | Git LFS install + `.gitattributes` consistency |
| `repo-guard ignore check` | Configured `ignored_paths` and near-match diagnostics |
| `repo-guard history check --range <base>..<head>` | Blobs introduced in a revision range (strict by default) |
| `repo-guard history check --range <base>..<head> --advisory` | Unresolved baselines warn instead of fail; blob policy violations still fail |
| `repo-guard history check --all` | Blobs reachable from `HEAD` |
| `repo-guard hook pre-commit` | Hook entry: staged-index check |
| `repo-guard hook pre-push` | Hook entry: outgoing objects from stdin (standalone only) |
| `repo-guard integrate pre-commit --print` | Reviewable pre-commit snippet (manual install) |
| `repo-guard integrate husky --print` | Reviewable Husky snippet |
| `repo-guard integrate lefthook --print` | Reviewable Lefthook snippet |
| `repo-guard integrate ci --print` | Pull-request CI example snippet |

Global option: `--format text|json` on all commands.

Run `repo-guard integrate detect` only if you prefer the `integrate` subcommand tree; behavior matches `repo-guard detect`.

## Git LFS checks and repair

Default thresholds (see `.repo-guard.toml.example`):

- `lfs_threshold_mib = 50` — a **regular** (non–LFS pointer) staged blob at or above this size **fails** policy and must be moved to Git LFS. Sizes from 50 MiB up to (but not including) the ceiling use the `blob.lfs_required` violation and repair commands.
- `github_safety_ceiling_mib = 95` — a separate, stricter rule for regular blobs at or above **95 MiB** (`blob.github_ceiling`). Both thresholds block commits; the ceiling message highlights Git-host size risk.

**Regular Git blobs between 50 MiB and 94 MiB do not pass** — they must become LFS pointers before commit.

When a large regular blob is staged, Repo Guard prints:

```powershell
git lfs track --filename "path/to/large.zip"
git add .gitattributes
git add --renormalize -- "path/to/large.zip"
repo-guard check
```

Repo Guard does not auto-edit `.gitattributes` or run `git lfs track` for you.

## Outgoing history and pre-push validation

**Standalone** `.githooks/pre-push`:

- Reads Git's ref update list from stdin
- Validates outgoing blobs in the push range
- Preserves stdin for `git lfs pre-push`
- Skips ref deletions

**Strict baseline resolution (default):** if a remote baseline SHA is not available locally, validation fails with an actionable error (exit 2). Fetch the ref or use an explicit range you can resolve.

**`--advisory`** on `history check` only: when a remote baseline ref cannot be resolved locally, Repo Guard emits a warning instead of failing with `history.unresolved_baseline`. **Outgoing blob policy violations (for example `blob.lfs_required`) still fail** regardless of `--advisory`. Use advisory mode only when you accept incomplete baseline coverage.

## Configuration

```bash
repo-guard init --apply
```

Or copy `.repo-guard.toml.example` to `.repo-guard.toml`.

In **integration** mode (pre-commit, Husky, Lefthook, etc.), `init --apply` may still create `.repo-guard.toml` and run `git lfs install --local` while **skipping** standalone `.githooks` setup and `core.hooksPath` changes. Run `repo-guard detect` to see which actions apply.

## Integration with existing hook managers

```bash
repo-guard detect
repo-guard integrate pre-commit --print
```

Run **`repo-guard check` after** formatters or fixers that mutate and re-stage files.

### pre-commit framework

After this repository is published, consumers can reference:

```yaml
repos:
  - repo: https://github.com/kd3f/repo-guard
    rev: v0.2.0
    hooks:
      - id: repo-guard-check
```

Canonical hook id: **`repo-guard-check`** (see `.pre-commit-hooks.yaml`).

The manifest uses `language: system` and `entry: repo-guard check`. Consumers must **install `repo-guard` separately** (for example `cargo install --path . --locked`) and have it on **`PATH`** when pre-commit runs — the framework does not bundle the binary.

## CI enforcement

Local hooks can be bypassed. Run history checks in CI:

`history check` resolves Git objects locally. The range endpoints (for example `origin/main` and `HEAD`) must exist in the runner's object database — run `git fetch` in the job when needed. **Shallow checkouts** often need `fetch-depth: 0` in GitHub Actions (as in `.github/workflows/repo-guard-ci.yml`) or an explicit `git fetch` of the base and head SHAs before calling Repo Guard.

```bash
# Push workflow (example)
repo-guard history check --range origin/main..HEAD

# Pull-request workflow (example)
repo-guard history check --range "$BASE_SHA..$HEAD_SHA"
```

`repo-guard integrate ci --print` emits a **GitHub Actions pull-request** example:

```yaml
repo-guard history check --range "${{ github.event.pull_request.base.sha }}..${{ github.sha }}"
```

Adapt the range for push workflows. See `.github/workflows/repo-guard-ci.yml` in this repository.

## Supported platforms

| OS | Support |
|---|---|
| Windows | Tested in CI (`windows-latest`) |
| macOS | Tested in CI (`macos-latest`) |
| Linux | Tested in CI (`ubuntu-latest`) |

Git hooks use POSIX shell in `.githooks/`; line endings are normalized via `.gitattributes`.

## Limitations (v0.2.0)

- Not a formatter, linter, test runner, or commit-msg tool
- Does not edit Husky, Lefthook, or pre-commit config automatically
- No pre-push enforcement through external hook managers
- No automatic history rewrite or LFS migration
- No credential storage or auto-push
- Early public release — pin versions and read release notes before production rollout

## Troubleshooting

| Symptom | What to try |
|---|---|
| `repo-guard` not found in hook or pre-commit | `cargo install --path . --locked` and add Cargo `bin` to `PATH`; Unix `.githooks` exit **3** when missing |
| `history check` fails on baseline | `git fetch` the remote ref; widen checkout depth; or `--advisory` only if incomplete checks are acceptable |
| `init --apply` skips `.githooks` | External hook manager detected — config (and often `git lfs install --local`) may still apply; use `integrate * --print` |
| `init --apply` blocked entirely | Hard hook conflict (for example existing `core.hooksPath`) — resolve manually |
| Hooks missing after clone | Run `repo-guard install --apply` (`core.hooksPath` is local config) |
| Large file still blocked | Follow printed `repair.command` steps (`git lfs track`, `--renormalize`, re-run `check`) |

## Contributing, security, and license

- [CONTRIBUTING.md](CONTRIBUTING.md) — development setup and standards
- [SECURITY.md](SECURITY.md) — vulnerability reporting
- [docs/releases/v0.2.0.md](docs/releases/v0.2.0.md) — v0.2.0 release notes

Licensed under **Apache-2.0** — see [LICENSE](LICENSE).

Source repository: [https://github.com/kd3f/repo-guard](https://github.com/kd3f/repo-guard) (confirm repository name with the maintainer before publishing forks or docs).
