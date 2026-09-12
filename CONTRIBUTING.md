# Contributing to ModelProof Contracts

Thank you for your interest in contributing to **ModelProof**! This guide outlines how to contribute code, report issues, and propose improvements to our Soroban smart contracts.

---

## Code of Conduct

All contributors and community participants are expected to adhere to our [Code of Conduct](CODE_OF_CONDUCT.md). Please treat others with respect and empathy.

---

## Development Setup

### Prerequisites

1. **Rust Toolchain**: Install stable Rust (1.80+ recommended):
   ```bash
   rustup default stable
   ```
2. **WASM Target**: Install the target for Soroban contract compilation:
   ```bash
   rustup target add wasm32-unknown-unknown
   ```
3. **Soroban CLI** (Optional, for local network simulation and contract deployment):
   ```bash
   cargo install --locked soroban-cli
   ```

### Building the Workspace

Clone the repository and build:
```bash
git clone https://github.com/ModelProof/modelproof-contracts.git
cd modelproof-contracts
cargo build
```

To build optimized WASM bytecode:
```bash
cargo build --target wasm32-unknown-unknown --release
```

---

## Testing & Quality Assurance

All contributions must maintain high standards of correctness, security, and invariant preservation.

### Running Unit Tests

Run the test suite using:
```bash
cargo test --lib
```

> **Note**: Always include `--lib` when testing locally on environments where building multi-crate cdylib test harnesses might encounter platform linker limitations.

### Code Formatting

All code must be formatted using `rustfmt`:
```bash
cargo fmt --all -- --check
```
To auto-format:
```bash
cargo fmt --all
```

### Linting

Run Clippy to catch common errors and non-idiomatic patterns:
```bash
cargo clippy --all-targets -- -D warnings
```

---

## Development Guidelines

1. **`#![no_std]` Compatibility**:
   - All Soroban contract crates must operate in `#![no_std]` mode.
   - Use `soroban_sdk` types (`Vec`, `Map`, `String`, `Address`, `Symbol`, `Option`) instead of Rust `std` collections.
2. **Storage and Mutation Invariants**:
   - Always validate inputs and authorization upfront before mutating contract persistent storage.
   - Ensure operations never leave partial or inconsistent state upon error return.
3. **Explicit Error Types**:
   - Every failure condition must return an explicit, descriptive variant of `RegistryError`.
4. **Preserve Existing Tests**:
   - Never remove or break existing unit tests unless an intentional breaking change is approved by maintainers.
   - Every new feature or bug fix must include dedicated unit tests covering both positive and negative/rejection scenarios.

---

## Contribution Workflow

1. **Find or Open an Issue**: Discuss large changes or new features in a GitHub issue before opening a pull request.
2. **Fork and Branch**: Create a feature branch from `main`:
   ```bash
   git checkout -b feat/my-new-feature
   ```
3. **Make Your Changes**: Keep commits focused and descriptive (using Conventional Commits format, e.g. `feat:`, `fix:`, `docs:`, `test:`).
4. **Verify Locally**:
   - Run `cargo fmt --all -- --check`
   - Run `cargo test --lib`
   - Run `cargo clippy` (if supported)
   - Ensure the WASM release build succeeds
5. **Submit a Pull Request**: Fill out the pull request template completely and link any related issues.

---

## Reporting Bugs and Feature Requests

- Use GitHub issues to report bugs or request features.
- Please use the provided issue templates:
  - [Bug Report](.github/ISSUE_TEMPLATE/bug_report.md)
  - [Feature Request](.github/ISSUE_TEMPLATE/feature_request.md)
- For security vulnerabilities, please refer to [SECURITY.md](SECURITY.md) and do not open a public issue.
