# ModelProof Contracts

`modelproof-contracts` is an open-source suite of Rust smart contracts for [Soroban](https://soroban.stellar.org/) on the [Stellar](https://stellar.org/) network.

ModelProof provides an AI provenance and verification platform. These smart contracts enable decentralized registration, verification, and tracking of AI-related artifacts throughout their lifecycle.

## Tracked Artifacts

- **Datasets**: Provenance records for training, fine-tuning, and evaluation datasets.
- **AI Models**: Weights, architectures, and base model registrations.
- **Model Versions**: Checkpoints, fine-tunes, and minor/major release versions.
- **Training Runs**: Verifiable records of compute, hyperparameter configs, and environment metadata.
- **Evaluations**: Benchmark outputs, validation scores, and safety red-teaming reports.
- **Published Artifacts**: Final deployment records and API binding verifications.

## Architecture & Structure

```
modelproof-contracts/
├── Cargo.toml               # Workspace cargo manifest
├── contracts/
│   └── registry/            # Core artifact provenance registry contract
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs       # Contract implementation & unit tests
└── README.md
```

## Contract Interface

### `ModelProofRegistry`

- **`register_artifact(owner: Address, artifact_id: String, artifact_hash: String, artifact_type: String) -> Result<ArtifactRecord, RegistryError>`**
  Registers a new AI artifact record, records block timestamp, and emits an `artifact_registered` event. Returns `RegistryError::AlreadyExists` if `artifact_id` is registered.

- **`get_artifact(artifact_id: String) -> Option<ArtifactRecord>`**
  Retrieves the registered `ArtifactRecord` by ID.

## Building and Testing

### Prerequisites

- Rust toolchain (stable)
- WASM target: `rustup target add wasm32-unknown-unknown`
- `soroban-cli` (optional for local deployment)

### Running Unit Tests

```bash
cargo test
```

### Formatter

```bash
cargo fmt --all --check
```

## License

Apache-2.0 or MIT
