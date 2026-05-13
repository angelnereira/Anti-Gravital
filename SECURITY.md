# Security Policy

## Supported Versions

Anti-Gravital is currently in pre-release. Security fixes are applied to the `main` branch immediately and will be backported to minor releases once v1.0 is tagged.

| Version | Supported |
|---|---|
| `main` (pre-release) | Yes |
| < 1.0 | No backports |

## Reporting a Vulnerability

Do not file a public GitHub issue for security vulnerabilities.

Send a report to **security@gravital.dev** with the subject line:

```
Security: Anti-Gravital — <brief description>
```

Include in your report:

- A description of the vulnerability and its potential impact
- Steps to reproduce (a minimal proof-of-concept if possible)
- Affected versions or commit ranges
- Any suggested mitigations

### What to Expect

- **Acknowledgment** within 48 hours of receipt.
- **Triage** within 5 business days — we will confirm whether the report is accepted and assign a severity.
- **Fix and disclosure timeline** — critical issues targeting the network-facing Shield layer will be fixed within 14 days. Lower-severity issues within 90 days.
- **Credit** — reporters who follow responsible disclosure are credited in the release notes and the `SECURITY.md` changelog below.

### Out of Scope

- Vulnerabilities in dependencies that are already tracked by RustSec (`cargo audit`)
- Denial-of-service attacks requiring authenticated access
- Theoretical attacks with no practical exploit path

## Security Design Notes

### The Shield

The Shield (Tower middleware) is the security boundary of Anti-Gravital. It is the only layer that touches untrusted network data. Design decisions relevant to security:

- **TLS**: Implemented with `rustls` — a pure-Rust TLS 1.3 implementation. No OpenSSL dependency and no C FFI in the TLS path.
- **JWT**: Signatures are verified with Ed25519 via the `ring` crate before any handler code runs.
- **Schema validation**: Request bodies are validated against compiled `.ag` contracts in the Shield. Malformed requests are rejected before reaching business logic.
- **Rate limiting**: Token-bucket rate limiting per client IP via `governor`. Prevents request flooding before it reaches Axum handlers.
- **Memory safety**: The entire codebase is written in safe Rust. `unsafe` blocks, if any, are documented with a `// SAFETY:` invariant comment and audited on every change.

### Dependencies

Dependency security is tracked via `cargo audit`. The CI pipeline runs `cargo audit` on every pull request and on a nightly schedule against `main`.

## Disclosure History

No vulnerabilities have been disclosed to date.
