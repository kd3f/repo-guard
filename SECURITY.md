# Security policy

## Supported versions

| Version | Supported |
|---|---|
| 0.2.x | Yes — current public release line |
| Earlier or unreleased | No |

Security fixes are applied to the latest 0.2.x release when practical.

## Reporting a vulnerability

**Do not** open a public GitHub issue for security-sensitive reports.

**Preferred channel:** [GitHub private vulnerability reporting](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing-information-about-vulnerabilities/privately-reporting-a-security-vulnerability) on this repository (**Security** → **Report a vulnerability**).

The maintainer must **enable and verify** private vulnerability reporting in repository settings before the first public launch. Until that is active, use **Security → Report a vulnerability** only after the feature is turned on; otherwise contact the maintainer through GitHub (for example a minimal private communication request) without posting exploit details in public issues.

We do not list a dedicated security email in this document because one is not officially provisioned for the project.

### What to include

- Affected version or commit
- Component (CLI command, hook, CI example, etc.)
- Clear reproduction steps
- Impact assessment (data exposure, credential handling, arbitrary command execution, etc.)
- Suggested fix if you have one (optional)

### What we need from reporters

- Good-faith disclosure — allow time to investigate and patch before public discussion
- Avoid exploiting issues beyond what is needed to demonstrate impact
- Do not include live secrets or customer data in reports

## Disclosure

We coordinate disclosure after a fix or mitigation when possible. We do not guarantee specific response or patch timelines.

Public release notes for security fixes will describe impact at a high level without unnecessary exploit detail.

## Scope notes

Repo Guard:

- Runs local Git and Git LFS commands; it does not store credentials
- Does not push, create remotes, or modify hosting accounts
- Is not a network service — threat model is primarily local repository access and CI runner execution

Out of scope for this policy: vulnerabilities in Git, Git LFS, Rust, or third-party hook managers themselves (report those to their respective projects).
