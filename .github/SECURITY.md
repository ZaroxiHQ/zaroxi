# Security Policy

## Supported versions

Zaroxi Studio is under active development and has not yet reached a stable
release. Security fixes are applied to the `main` branch and included in the next
tagged release.

| Version | Supported          |
| ------- | ------------------ |
| `main`  | :white_check_mark: |
| tagged pre-releases (`0.x`) | latest only |

## Reporting a vulnerability

**Please do not open a public issue for security vulnerabilities.**

Report vulnerabilities privately via GitHub's private vulnerability reporting:

- Go to the [Security advisories page](https://github.com/ZaroxiHQ/zaroxi/security/advisories/new)
  and open a new draft advisory.

Please include:

- A description of the vulnerability and its impact.
- Steps to reproduce (proof-of-concept if possible).
- Affected version / commit and platform (Linux, macOS, Windows).
- Any suggested remediation.

## What to expect

- **Acknowledgement:** we aim to acknowledge new reports within 5 business days.
- **Assessment:** we will investigate, confirm the issue, and agree on a fix and
  disclosure timeline with you.
- **Fix & disclosure:** once a fix is available we will publish a release and a
  security advisory crediting the reporter (unless you prefer to remain
  anonymous).

## Dependency security

Dependencies are continuously monitored:

- `cargo audit` and `cargo deny check` run on every push, pull request, and on a
  weekly schedule (see `.github/workflows/security-audit.yml`).
- CodeQL static analysis runs on the Rust codebase (see
  `.github/workflows/codeql.yml`).
- Justified, documented advisory exceptions live in `.cargo/audit.toml` and
  `deny.toml`, each with a rationale and an upgrade path.
