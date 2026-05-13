# Contributing to Anti-Gravital

Thank you for your interest in contributing. This document covers the development workflow, code standards, and process for submitting changes.

## Development Setup

### Prerequisites

- Rust 1.75 or newer — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Git 2.35 or newer
- On Linux: `libssl-dev`, `pkg-config`
- On macOS: Xcode Command Line Tools

### Clone and Build

```sh
git clone https://github.com/gravital-labs/anti-gravital
cd anti-gravital

rustup component add rustfmt clippy

cargo build --workspace
```

### Running Tests

```sh
# All tests
cargo test --workspace

# Single crate
cargo test -p ag-core
cargo test -p ag-dsl
```

### Running Benchmarks

```sh
cargo bench -p ag-core
```

## Code Style

All Rust code must pass `cargo fmt` and `cargo clippy` with no warnings.

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

Key conventions:

- Use `thiserror` for error types. Do not use `Box<dyn Error>` in library code.
- Prefer explicit lifetime annotations over elision when it aids readability.
- Every public type and function must have a doc comment.
- Use `#[allow(dead_code)]` only at the item level and only with a comment explaining why the item is not yet called.
- Unsafe blocks must have a `// SAFETY:` comment explaining the invariants being upheld.
- Do not add comments that restate what the code does. Only comment the *why* when it is non-obvious.

## Commit Messages

This project uses the Conventional Commits specification (https://www.conventionalcommits.org/).

Format:
```
<type>(<scope>): <short description>

[optional body]

[optional footer(s)]
```

Types: `feat`, `fix`, `perf`, `refactor`, `test`, `docs`, `build`, `ci`, `chore`

Scopes: `shield`, `core`, `dsl`, `cli`, `auth`, `data`, `realtime`, `cache`, `storage`, `ui`, `observability`, `wasm`, `bench`, `docs`

Examples:
```
feat(shield): add JWT Ed25519 verification Tower layer

fix(dsl): handle nested array types in semantic checker

perf(core): eliminate per-request allocation in ValidatedBody extractor

docs(schema): add enum and relation definition examples
```

Rules:

- Subject line must not exceed 72 characters.
- Subject line must not end with a period.
- Body lines must not exceed 100 characters.
- Use imperative mood ("add" not "adds" or "added").
- Breaking changes must include `BREAKING CHANGE:` in the footer.

## Pull Request Process

1. Fork the repository and create a feature branch from `main`.

2. Branch naming: `<type>/<short-description>`, for example `feat/shield-jwt-layer` or `fix/dsl-array-types`.

3. Keep pull requests focused. One concern per PR. Large changes should be a sequence of smaller PRs.

4. Before submitting:
   - All tests pass: `cargo test --workspace`
   - No lint warnings: `cargo clippy --workspace --all-targets -- -D warnings`
   - Code is formatted: `cargo fmt --all`
   - New public APIs have documentation
   - New behaviors have tests

5. Pull request description must explain:
   - What the change does
   - Why it is needed
   - How it was tested
   - Any performance implications

6. All CI checks must pass before a PR can be merged.

7. At least one maintainer review is required.

8. Merge strategy: squash merge for feature branches, merge commit for release branches.

## Reporting Issues

Use GitHub Issues at https://github.com/gravital-labs/anti-gravital/issues.

For bug reports, include:

- Operating system and version
- Rust version (`rustc --version`)
- Steps to reproduce
- Expected behavior
- Actual behavior
- Any relevant logs or error messages

## Security Vulnerabilities

Do not file a public issue for security vulnerabilities. See [SECURITY.md](SECURITY.md) for the responsible disclosure process.
