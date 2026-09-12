## Description
Provide a concise explanation of what this pull request changes and why.

## Related Issues
Closes #(issue number)

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Refactoring or code cleanup
- [ ] Documentation update
- [ ] CI / tooling improvement

## Invariants & Security Checklist
- [ ] All inputs and authorizations are verified before any persistent storage mutations.
- [ ] Operations return explicit, typed errors (`RegistryError`) on invalid input or state.
- [ ] Existing provenance history and attestations remain immutable upon lifecycle changes.
- [ ] All contracts maintain `#![no_std]` compliance.

## Testing & Verification
- [ ] `cargo fmt --all -- --check` passes cleanly.
- [ ] `cargo test --lib` passes all unit tests.
- [ ] `cargo clippy` passes without warnings.
- [ ] WASM release build (`cargo build --target wasm32-unknown-unknown --release`) succeeds.
- [ ] New unit tests have been added covering new logic and failure modes.
