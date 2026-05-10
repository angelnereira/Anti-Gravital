# Contributing to Anti-Gravital

Thank you for your interest in contributing. This document covers the development workflow, code standards, and process for submitting changes.

## Development Setup

### Prerequisites

- Rust 1.75 or newer. Install via rustup: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- Go 1.21 or newer. Install from https://go.dev/dl/
- Git 2.35 or newer
- On Linux: `libssl-dev`, `pkg-config`
- On macOS: Xcode Command Line Tools

### Clone and Build

```sh
git clone https://github.com/angelnereira/anti-gravital
cd anti-gravital

# Install Rust tooling
rustup component add rustfmt clippy

# Build everything
cargo build
cd ag-runtime && go build ./...
```

### Running Tests

```sh
# Rust tests
cargo test

# Go tests
cd ag-runtime && go test ./...

# All tests
cargo test && (cd ag-runtime && go test ./...)
```

## Code Style

### Rust

All Rust code must pass `cargo fmt` and `cargo clippy` with no warnings.

```sh
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
```

Key conventions:
- Use `thiserror` for error types. Do not use `Box<dyn Error>` in library code.
- Prefer explicit lifetime annotations over elision when it aids readability.
- Every public type and function must have a doc comment.
- Use `#[allow(dead_code)]` only at the item level and only with a comment explaining why the code exists but is not yet called.
- Unsafe blocks must have a `// SAFETY:` comment explaining the invariants being upheld.

### Go

All Go code must pass `gofmt` and `go vet`.

```sh
gofmt -w ./ag-runtime/...
cd ag-runtime && go vet ./...
```

Key conventions:
- Follow the standard Go project layout conventions.
- Error values must be wrapped with `fmt.Errorf("context: %w", err)` to preserve the error chain.
- Goroutines must have a defined lifecycle: every goroutine started must have a corresponding shutdown path.
- Exported types must have godoc comments.
- CGO blocks must have a comment explaining the memory ownership model.

## Commit Messages

This project uses the Conventional Commits specification (https://www.conventionalcommits.org/).

Format:
```
<type>(<scope>): <short description>

[optional body]

[optional footer(s)]
```

Types:
- `feat`: a new feature
- `fix`: a bug fix
- `perf`: a change that improves performance
- `refactor`: a code change that is neither a feature nor a fix
- `test`: adding or updating tests
- `docs`: documentation changes only
- `build`: changes to the build system or dependencies
- `ci`: changes to CI configuration
- `chore`: maintenance tasks

Scopes: `bus`, `shield`, `brain`, `dsl`, `cli`, `runtime`, `bench`, `docs`

Examples:
```
feat(bus): implement ring buffer slot claiming via CAS

fix(shield): correct TLS certificate reload race condition

perf(bus): replace spin loop with futex wait in slot consumer

docs(schema): add enum definition examples to schema reference
```

Rules:
- Subject line must not exceed 72 characters.
- Subject line must not end with a period.
- Body lines must not exceed 100 characters.
- Use imperative mood in the subject ("add" not "adds" or "added").
- Breaking changes must include `BREAKING CHANGE:` in the footer.

## Pull Request Process

1. Fork the repository and create a feature branch from `main`.

2. Branch naming: `<type>/<short-description>`, for example `feat/bus-futex-notifier` or `fix/shield-tls-reload`.

3. Keep pull requests focused. A PR should address one concern. Large changes should be broken into a sequence of smaller PRs.

4. Before submitting:
   - All tests pass: `cargo test && (cd ag-runtime && go test ./...)`
   - No lint warnings: `cargo clippy --all-targets -- -D warnings && (cd ag-runtime && go vet ./...)`
   - Code is formatted: `cargo fmt --all && gofmt -w ./ag-runtime/...`
   - New public APIs have documentation
   - New behaviors have tests

5. Pull request description must explain:
   - What the change does
   - Why it is needed
   - How it was tested
   - Any performance implications

6. All CI checks must pass before a PR can be merged.

7. At least one maintainer review is required. Address all review comments before requesting re-review.

8. Merge strategy: squash merge for feature branches, merge commit for release branches.

## Reporting Issues

Use GitHub Issues. For security vulnerabilities, do not file a public issue. Instead, email security@gravital.dev with the subject line "Security: Anti-Gravital".

For bug reports, include:
- Operating system and version
- Rust version (`rustc --version`)
- Go version (`go version`)
- Steps to reproduce
- Expected behavior
- Actual behavior
- Any relevant logs or error messages
