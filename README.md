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
  Registers a new AI artifact. Requires owner authentication. Defaults to `Active` status. Emits `artifact_registered`.
  - Validates `artifact_id`, `artifact_hash`, and `artifact_type` are non-empty.
  - Rejects duplicate artifact IDs (`AlreadyExists`).

- **`get_artifact(artifact_id) -> Option<ArtifactRecord>`**
  Retrieves a registered `ArtifactRecord` by ID (including revoked or superseded artifacts).

- **`revoke_artifact(artifact_id, revocation_reason_hash) -> Result<ArtifactRecord, RegistryError>`**
  Revokes an artifact. Requires owner authentication. Sets status to `Revoked`, records `revoked_at` timestamp and `revocation_reason_hash`. Preserves the full record and all provenance/attestation history. Emits `artifact_revoked`.
  - Rejects if already revoked (`ArtifactAlreadyRevoked`).
  - Rejects if already superseded (`CannotRevokeSupersededArtifact`).
  - Rejects empty revocation reason hash (`EmptyRevocationReason`).

- **`supersede_artifact(old_artifact_id, replacement_artifact_id, provenance_relation_id) -> Result<ArtifactRecord, RegistryError>`**
  Marks an artifact as superseded by a replacement. Requires old artifact owner authentication. Sets old artifact status to `Superseded`, auto-creates a `PreviousVersion` provenance relation from replacement to old artifact. Emits `artifact_superseded`.
  - Rejects self-superseding (`SelfReferencingNotAllowed`).
  - Rejects empty relation ID (`EmptyRelationId`).
  - Rejects if old artifact is revoked (`CannotSupersedeRevokedArtifact`).
  - Rejects if old artifact is already superseded (`ArtifactAlreadySuperseded`).
  - Rejects if replacement artifact does not exist (`ReplacementArtifactNotFound`).
  - Requires replacement artifact to be in `Active` status (`ReplacementArtifactNotActive`).
  - Rejects duplicate relation ID (`RelationAlreadyExists`).

#### Provenance Relationship Functions

- **`add_provenance_relation(relation_id, source_artifact_id, target_artifact_id, relation_type) -> Result<ProvenanceRelation, RegistryError>`**
  Creates a provenance link between two artifacts. Requires source artifact owner authentication. Emits `provenance_relation_added`.
  - Validates `relation_id` is non-empty (`EmptyRelationId`).
  - Rejects self-referencing relations (`SelfReferencingNotAllowed`).
  - Rejects duplicate relation IDs (`RelationAlreadyExists`).
  - Validates source artifact exists (`SourceArtifactNotFound`) and is not revoked (`SourceArtifactRevoked`).
  - Validates target artifact exists (`TargetArtifactNotFound`) and is not revoked (`TargetArtifactRevoked`).

- **`get_provenance_relation(relation_id) -> Option<ProvenanceRelation>`**
  Retrieves a `ProvenanceRelation` by its unique relation ID.

- **`get_artifact_relations(artifact_id) -> Vec<ProvenanceRelation>`**
  Retrieves all provenance relationships associated with an artifact (as source or target).

#### Attestation Functions

- **`add_attestation(attestation_id, artifact_id, attester, attestation_type, evidence_hash) -> Result<Attestation, RegistryError>`**
  Attaches a verifiable attestation to a registered artifact. Any authorized Stellar address may attest independent of ownership. Emits `attestation_added`.
  - Validates `attestation_id` is non-empty (`EmptyAttestationId`).
  - Validates `evidence_hash` is non-empty (`EmptyEvidenceHash`).
  - Rejects duplicate attestation IDs (`AttestationAlreadyExists`).
  - Validates artifact exists (`ArtifactNotFound`).
  - Rejects attestations on revoked artifacts (`CannotAttestRevokedArtifact`).

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

## Registry Error Codes

| Code | Variant | Invariant / Failure Condition |
|---|---|---|
| `1` | `AlreadyExists` | Artifact ID is already registered |
| `2` | `NotFound` | Query or target record not found |
| `3` | `SourceArtifactNotFound` | Source artifact does not exist when creating relation |
| `4` | `TargetArtifactNotFound` | Target artifact does not exist when creating relation |
| `5` | `SelfReferencingNotAllowed` | Self-referencing provenance relation or superseding attempted |
| `6` | `RelationAlreadyExists` | Provenance relation ID already exists |
| `7` | `ArtifactNotFound` | Target artifact does not exist for attestation or revocation |
| `8` | `AttestationAlreadyExists` | Attestation ID already exists |
| `9` | `EmptyEvidenceHash` | Attestation evidence hash is empty string |
| `10` | `ArtifactAlreadyRevoked` | Revoking an already-revoked artifact |
| `11` | `EmptyRevocationReason` | Revocation reason hash is empty string |
| `12` | `ReplacementArtifactNotFound` | Replacement artifact does not exist when superseding |
| `13` | `EmptyArtifactId` | Artifact ID is empty string on registration |
| `14` | `EmptyArtifactHash` | Artifact hash is empty string on registration |
| `15` | `EmptyArtifactType` | Artifact type is empty string on registration |
| `16` | `EmptyRelationId` | Relation ID is empty string on relation or superseding |
| `17` | `SourceArtifactRevoked` | Provenance relation source artifact is Revoked |
| `18` | `TargetArtifactRevoked` | Provenance relation target artifact is Revoked |
| `19` | `EmptyAttestationId` | Attestation ID is empty string |
| `20` | `CannotAttestRevokedArtifact` | Attestation attempted on a Revoked artifact |
| `21` | `CannotRevokeSupersededArtifact` | Revocation attempted on a Superseded artifact |
| `22` | `CannotSupersedeRevokedArtifact` | Superseding attempted on a Revoked artifact |
| `23` | `ArtifactAlreadySuperseded` | Superseding attempted on an already-superseded artifact |
| `24` | `ReplacementArtifactNotActive` | Replacement artifact for superseding is not in Active status |

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
