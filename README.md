# ModelProof Contracts

`modelproof-contracts` is an open-source suite of Rust smart contracts for [Soroban](https://soroban.stellar.org/) on the [Stellar](https://stellar.org/) network.

ModelProof provides an AI provenance and verification platform. These smart contracts enable decentralized registration, verification, tracking, and lifecycle management of AI-related artifacts, their provenance relationships, and independent attestations.

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

- **`register_artifact(owner, artifact_id, artifact_hash, artifact_type) -> Result<ArtifactRecord, RegistryError>`**
  Registers a new AI artifact. Requires owner authentication. Defaults to `Active` status. Emits `artifact_registered`. Rejects duplicate IDs.

- **`get_artifact(artifact_id) -> Option<ArtifactRecord>`**
  Retrieves a registered `ArtifactRecord` by ID (including revoked or superseded artifacts).

- **`revoke_artifact(artifact_id, revocation_reason_hash) -> Result<ArtifactRecord, RegistryError>`**
  Revokes an artifact. Requires owner authentication. Sets status to `Revoked`, records `revoked_at` timestamp and `revocation_reason_hash`. Preserves the full record and all provenance/attestation history. Emits `artifact_revoked`.

- **`supersede_artifact(old_artifact_id, replacement_artifact_id, provenance_relation_id) -> Result<ArtifactRecord, RegistryError>`**
  Marks an artifact as superseded by a replacement. Requires old artifact owner authentication. Sets old artifact status to `Superseded`, auto-creates a `PreviousVersion` provenance relation from replacement to old artifact. Emits `artifact_superseded`.

#### Provenance Relationship Functions

- **`add_provenance_relation(relation_id, source_artifact_id, target_artifact_id, relation_type) -> Result<ProvenanceRelation, RegistryError>`**
  Creates a provenance link between two artifacts. Requires source artifact owner authentication. Validates both artifacts exist, prevents self-referencing, enforces unique relation IDs. Emits `provenance_relation_added`.

- **`get_provenance_relation(relation_id) -> Option<ProvenanceRelation>`**
  Retrieves a `ProvenanceRelation` by its unique relation ID.

- **`get_artifact_relations(artifact_id) -> Vec<ProvenanceRelation>`**
  Retrieves all provenance relationships associated with an artifact (as source or target).

#### Attestation Functions

- **`add_attestation(attestation_id, artifact_id, attester, attestation_type, evidence_hash) -> Result<Attestation, RegistryError>`**
  Attaches a verifiable attestation to a registered artifact. Any authorized Stellar address may attest independent of ownership. Requires non-empty evidence hash. Emits `attestation_added`.

- **`get_attestation(attestation_id) -> Option<Attestation>`**
  Retrieves an `Attestation` record by its unique attestation ID.

- **`get_artifact_attestations(artifact_id) -> Vec<Attestation>`**
  Retrieves all attestations associated with a given artifact ID.

## Artifact Lifecycle

```
            register_artifact
                  │
                  ▼
            ┌─────────┐
            │  Active  │
            └────┬─────┘
                 │
       ┌─────────┴────────┐
       │                  │
  revoke_artifact   supersede_artifact
       │                  │
       ▼                  ▼
   ┌─────────┐      ┌────────────┐
   │ Revoked │      │ Superseded │
   └─────────┘      └────────────┘
```

All provenance relationships and attestations are preserved across all status transitions.

## Supported Provenance Relationship Types

| Variant | Meaning |
|---|---|
| `DerivedFrom` | Artifact derived from another (e.g. fine-tune from base weights) |
| `TrainedOn` | Model/run trained using a specific dataset |
| `EvaluatedWith` | Model/run evaluated using a specific benchmark |
| `PreviousVersion` | Sequential version chain link |
| `ProducedBy` | Artifact produced by a training or evaluation run |

## Supported Attestation Types

| Variant | Meaning |
|---|---|
| `Verified` | Independently verified by the attesting party |
| `Audited` | Formally audited for compliance or correctness |
| `Evaluated` | Evaluated for capability or performance |
| `Reproduced` | Results independently reproduced |
| `Endorsed` | Endorsed or recommended by the attesting party |

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
