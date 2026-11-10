# Security and static analysis

| Tool | Purpose | Status |
|---|---|---|
| `cargo audit` | RustSec advisory scanning for dependencies | Configured in `.github/workflows/repo-guard-ci.yml` |
| `cargo deny` | License and duplicate dependency policy | Optional follow-up |

Public-release CI runs `cargo audit` on every matrix job. Do not claim audit results until hosted CI has passed.

Local run:

```bash
cargo install cargo-audit --locked
cargo audit
```
