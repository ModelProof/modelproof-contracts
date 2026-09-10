# ModelProof Contracts

`modelproof-contracts` is an open-source suite of Rust smart contracts for [Soroban](https://soroban.stellar.org/) on the [Stellar](https://stellar.org/) network.

ModelProof provides an AI provenance and verification platform. These smart contracts enable decentralized registration, verification, and tracking of AI-related artifacts and their provenance relationships throughout their lifecycle.

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

#### Artifact Functions

- **`register_artifact(owner: Address, artifact_id: String, artifact_hash: String, artifact_type: String) -> Result<ArtifactRecord, RegistryError>`**
  Registers a new AI artifact record, records block timestamp, and emits an `artifact_registered` event. Returns `RegistryError::AlreadyExists` if `artifact_id` is registered.

- **`get_artifact(artifact_id: String) -> Option<ArtifactRecord>`**
  Retrieves the registered `ArtifactRecord` by ID.

#### Provenance Relationship Functions

- **`add_provenance_relation(relation_id: String, source_artifact_id: String, target_artifact_id: String, relation_type: RelationType) -> Result<ProvenanceRelation, RegistryError>`**
  Creates a provenance link from a source artifact to a target artifact.
  - Validates that source and target artifacts exist.
  - Enforces owner authentication of the source artifact.
  - Prevents self-referencing relationship links.
  - Ensures relation ID uniqueness.
  - Emits a `provenance_relation_added` event upon creation.

- **`get_provenance_relation(relation_id: String) -> Option<ProvenanceRelation>`**
  Retrieves a `ProvenanceRelation` record by its unique relation ID.

- **`get_artifact_relations(artifact_id: String) -> Vec<ProvenanceRelation>`**
  Retrieves all provenance relationships associated with a given artifact ID.

## Supported Provenance Relationship Types

- `DerivedFrom`: Artifact is derived from another artifact (e.g. fine-tuned model from base weights).
- `TrainedOn`: Model or training run trained using a specific dataset.
- `EvaluatedWith`: Model or run evaluated using a specific benchmark dataset.
- `PreviousVersion`: Sequential versioning chain link.
- `ProducedBy`: Artifact produced by a training or evaluation run.

## Building and Testing

### Prerequisites

- Rust toolchain (stable)
- WASM target: `rustup target add wasm32-unknown-unknown`
- `soroban-cli` (optional for local deployment)

### Running Unit Tests

```bash
cargo test --lib
```

### Formatter

```bash
cargo fmt --all
```

## License

Apache-2.0 or MIT
