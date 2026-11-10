# Contributing to Repo Guard

Thank you for helping improve Repo Guard. This project is a focused **Git and Git LFS file-safety policy engine** — please keep changes aligned with that scope.

## Development setup

1. Install [Rust](https://rustup.rs/) matching `rust-toolchain.toml` (currently **1.92.0** with `rustfmt` and `clippy`).
2. Install [Git](https://git-scm.com/) and [Git LFS](https://git-lfs.com/).
3. Clone the repository and build:

```bash
cargo build
cargo test --all-targets --all-features
```

Integration tests create temporary Git repositories locally. Most tests do **not** require network access.

Optional:

```bash
cargo install cargo-audit --locked
cargo audit
```

## Repository layout

```text
src/
  main.rs          # composition root
  cli/             # Clap definitions
  application/     # command workflows
  domain/          # policy rules and pure logic
  infrastructure/  # Git, filesystem, processes
  output/          # text/JSON rendering
  support/         # small shared helpers
tests/             # integration and unit tests
presets/           # opt-in .gitignore preset content
docs/              # public technical documentation
```

## Engineering standards

We aim for maintainable, testable Rust:

- **Rust idioms:** structs, enums, narrow traits, composition, explicit `Result` types
- **Separation:** domain logic stays free of Git subprocess details where practical
- **Functional core:** pure policy functions; side effects behind infrastructure
- **Typed errors:** prefer `thiserror` (or equivalent) over stringly-typed failures
- **No unnecessary abstractions** — solve the current problem clearly
- **No dead code:** remove unused imports, functions, dependencies, and commented-out blocks
- **Lint discipline:** avoid broad `#[allow(...)]`; treat Clippy warnings as failures in CI
- **Dependencies:** keep `Cargo.toml` minimal; drop unused crates promptly
- **File size guidance:** prefer production files under **300** lines; review at **350**; hard limit **500** except justified fixtures or generated code
- **Tests:** behavior changes need regression tests in the same change (unit and/or integration)
- **Documentation:** update README, `docs/`, and examples when CLI behavior or compatibility changes

## Quality gates

Before opening a pull request, run:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo build --release
```

CI also runs `cargo audit` and validates `.pre-commit-hooks.yaml`.

## Pull requests

- Keep diffs focused — one coherent change per PR when possible
- Describe user-visible behavior and link related issues
- Note which quality gates you ran and on which OS if relevant
- Do not include unrelated refactors or formatting churn
- Do not commit secrets, tokens, `.env` files, or local machine paths

## Bug reports

Use the [bug report issue template](.github/ISSUE_TEMPLATE/bug_report.yml). Include:

- Repo Guard version (`repo-guard --version`)
- OS and shell
- `git --version` and `git lfs version`
- Hook manager setup (pre-commit, Husky, standalone `.githooks`, etc.)
- Minimal reproduction steps
- Expected vs actual behavior
- Sanitized command output (text or JSON)

## Feature requests

Use the [feature request template](.github/ISSUE_TEMPLATE/feature_request.yml). Features should address real Git/Git LFS workflow pain — not general hook orchestration, formatting, or unrelated tooling.

## Hook ownership

Repo Guard must **not** silently take over another hook manager's configuration. Changes that touch init, install, detect, or integrate must preserve the ownership contract documented in `docs/compatibility-matrix.md`.

## License

By contributing, you agree that your contributions are licensed under the Apache License 2.0, consistent with the project LICENSE file.
