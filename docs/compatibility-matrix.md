# Repo Guard compatibility matrix

Evidence levels:

- **verified standalone hooks** — end-to-end tests using committed `.githooks/` wrappers and local bare remotes (`tests/integration_pre_push_acceptance.rs`, `tests/integration_lfs_stdin.rs`).
- **verified external staged-index integration** — pre-commit consumer repository tests (`tests/integration_precommit.rs`) with manifest validation.
- **verified history commands** — `repo-guard history check` exercised independently of hook manager (`tests/integration_pre_push_acceptance.rs`, `tests/integration_history_push.rs`).
- **printed snippet only** — reviewable output from `repo-guard integrate * --print`; not auto-installed.
- **unsupported** — intentionally out of scope for v0.2.

| Environment | Standalone pre-commit hook | Standalone pre-push hook | External staged-index integration | History check command | External pre-push integration |
|---|---|---|---|---|---|
| None (standalone) | verified standalone hooks | verified standalone hooks | n/a | verified history commands | n/a |
| pre-commit | n/a (external owner) | n/a | verified external staged-index integration | verified history commands | unsupported |
| Husky | n/a | n/a | printed snippet only | verified history commands | unsupported |
| Lefthook | n/a | n/a | printed snippet only | verified history commands | unsupported |
| Custom `core.hooksPath` | n/a | n/a | printed snippet only | verified history commands | unsupported |
| CI | n/a | n/a | n/a | verified history commands | n/a |

## Standalone setup

```bash
repo-guard detect
repo-guard init --apply   # only when detect recommends standalone mode
repo-guard check
```

Standalone `.githooks/pre-push` preserves Git LFS stdin via a temp-file tee (`tests/integration_lfs_stdin.rs`).

## pre-commit integration

Root manifest: `.pre-commit-hooks.yaml`

```yaml
repos:
  - repo: https://github.com/kd3f/repo-guard
    rev: v0.2.0
    hooks:
      - id: repo-guard-check
```

Run Repo Guard **after** hooks that mutate and re-stage files.

## CI enforcement

```bash
repo-guard history check --range <base>..<head>
```

Choose the range for your workflow:

- **Pull requests:** compare the PR base to the head commit (for example `${{ github.event.pull_request.base.sha }}..${{ github.sha }}` in GitHub Actions).
- **Push:** compare the remote default branch to the pushed commit (for example `origin/main..HEAD`).

`repo-guard integrate ci --print` shows a pull-request example only; adapt the range for push workflows.

Reference workflow: `.github/workflows/repo-guard-ci.yml`

## Limitations

- Husky, Lefthook, and custom hook paths: staged-index snippets are manual; outgoing-push is not enforced through those managers in v0.2.
- Unresolved remote baselines block in strict mode (default for hooks); use `--advisory` on `history check` only when incomplete validation is acceptable.
- macOS and Linux standalone hook execution is validated in hosted CI when the workflow runs; pre-commit integration tests run on Ubuntu, Windows, and macOS in CI.
