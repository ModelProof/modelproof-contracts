#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, String, Symbol, Vec,
};

/// Error types returned by the ModelProof registry contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RegistryError {
    AlreadyExists = 1,
    NotFound = 2,
    SourceArtifactNotFound = 3,
    TargetArtifactNotFound = 4,
    SelfReferencingNotAllowed = 5,
    RelationAlreadyExists = 6,
    ArtifactNotFound = 7,
    AttestationAlreadyExists = 8,
    EmptyEvidenceHash = 9,
    ArtifactAlreadyRevoked = 10,
    EmptyRevocationReason = 11,
    ReplacementArtifactNotFound = 12,
    EmptyArtifactId = 13,
    EmptyArtifactHash = 14,
    EmptyArtifactType = 15,
    EmptyRelationId = 16,
    SourceArtifactRevoked = 17,
    TargetArtifactRevoked = 18,
    EmptyAttestationId = 19,
    CannotAttestRevokedArtifact = 20,
    CannotRevokeSupersededArtifact = 21,
    CannotSupersedeRevokedArtifact = 22,
    ArtifactAlreadySuperseded = 23,
    ReplacementArtifactNotActive = 24,
}

/// Lifecycle status of a registered artifact.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ArtifactStatus {
    /// Artifact is registered and in active use.
    Active,
    /// Artifact has been revoked by its owner. History is preserved.
    Revoked,
    /// Artifact has been superseded by a newer artifact. History is preserved.
    Superseded,
}

/// Data structure representing an AI artifact provenance record.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactRecord {
    pub artifact_id: String,
    pub artifact_hash: String,
    pub artifact_type: String,
    pub owner: Address,
    pub created_at: u64,
    /// Current lifecycle status of the artifact.
    pub status: ArtifactStatus,
    /// Ledger timestamp at which the artifact was revoked or superseded, if applicable.
    pub revoked_at: Option<u64>,
    /// Hash of the document or message explaining the revocation reason, if applicable.
    pub revocation_reason_hash: Option<String>,
}

/// Category types for provenance relationships between artifacts.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RelationType {
    DerivedFrom,
    TrainedOn,
    EvaluatedWith,
    PreviousVersion,
    ProducedBy,
}

/// Data structure representing a provenance relationship between two artifacts.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProvenanceRelation {
    pub relation_id: String,
    pub source_artifact_id: String,
    pub target_artifact_id: String,
    pub relation_type: RelationType,
    pub created_at: u64,
}

/// Category types for artifact attestations.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttestationType {
    Verified,
    Audited,
    Evaluated,
    Reproduced,
    Endorsed,
}

/// Data structure representing a verifiable attestation attached to an artifact.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    pub attestation_id: String,
    pub artifact_id: String,
    pub attester: Address,
    pub attestation_type: AttestationType,
    pub evidence_hash: String,
    pub created_at: u64,
}

/// Storage keys for the contract state.
#[contracttype]
pub enum DataKey {
    Artifact(String),
    Relation(String),
    ArtifactRelations(String),
    Attestation(String),
    ArtifactAttestations(String),
}

/// Smart contract for registering and verifying AI artifact records, provenance
/// relationships, independent attestations, and artifact lifecycle status.
#[contract]
pub struct ModelProofRegistry;

#[contractimpl]
impl ModelProofRegistry {
    // -------------------------------------------------------------------------
    // Artifact Functions
    // -------------------------------------------------------------------------

    /// Registers a new AI artifact in the provenance registry.
    /// New artifacts default to `ArtifactStatus::Active`.
    pub fn register_artifact(
        env: Env,
        owner: Address,
        artifact_id: String,
        artifact_hash: String,
        artifact_type: String,
    ) -> Result<ArtifactRecord, RegistryError> {
        if artifact_id.is_empty() {
            return Err(RegistryError::EmptyArtifactId);
        }
        if artifact_hash.is_empty() {
            return Err(RegistryError::EmptyArtifactHash);
        }
        if artifact_type.is_empty() {
            return Err(RegistryError::EmptyArtifactType);
        }

        owner.require_auth();

        let key = DataKey::Artifact(artifact_id.clone());
        if env.storage().persistent().has(&key) {
            return Err(RegistryError::AlreadyExists);
        }

        let created_at = env.ledger().timestamp();

        let record = ArtifactRecord {
            artifact_id: artifact_id.clone(),
            artifact_hash,
            artifact_type,
            owner: owner.clone(),
            created_at,
            status: ArtifactStatus::Active,
            revoked_at: None,
            revocation_reason_hash: None,
        };

        env.storage().persistent().set(&key, &record);

        env.events().publish(
            (Symbol::new(&env, "artifact_registered"), artifact_id),
            owner,
        );

        Ok(record)
    }

    /// Retrieves an artifact provenance record by its artifact ID.
    pub fn get_artifact(env: Env, artifact_id: String) -> Option<ArtifactRecord> {
        let key = DataKey::Artifact(artifact_id);
        env.storage().persistent().get(&key)
    }

    /// Revokes an artifact. Requires authentication from the artifact owner.
    /// The artifact record is preserved with status set to `Revoked`.
    /// All provenance relationships and attestations remain queryable.
    ///
    /// # Arguments
    /// * `artifact_id` - ID of the artifact to revoke.
    /// * `revocation_reason_hash` - Non-empty hash of the revocation justification document.
    pub fn revoke_artifact(
        env: Env,
        artifact_id: String,
        revocation_reason_hash: String,
    ) -> Result<ArtifactRecord, RegistryError> {
        let key = DataKey::Artifact(artifact_id.clone());

        let mut record: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(RegistryError::ArtifactNotFound)?;

        // Authenticate as the artifact owner before any mutation
        record.owner.require_auth();

        if record.status == ArtifactStatus::Revoked {
            return Err(RegistryError::ArtifactAlreadyRevoked);
        }

        if record.status == ArtifactStatus::Superseded {
            return Err(RegistryError::CannotRevokeSupersededArtifact);
        }

        if revocation_reason_hash.is_empty() {
            return Err(RegistryError::EmptyRevocationReason);
        }

        let revoked_at = env.ledger().timestamp();

        record.status = ArtifactStatus::Revoked;
        record.revoked_at = Some(revoked_at);
        record.revocation_reason_hash = Some(revocation_reason_hash);

        env.storage().persistent().set(&key, &record);

        env.events()
            .publish((Symbol::new(&env, "artifact_revoked"), artifact_id), ());

        Ok(record)
    }

    /// Marks an artifact as superseded by a replacement artifact.
    /// Requires authentication from the owner of the old artifact.
    /// Automatically creates a `PreviousVersion` provenance relation from the
    /// replacement artifact to the old artifact and preserves all history.
    ///
    /// # Arguments
    /// * `old_artifact_id` - ID of the artifact being superseded.
    /// * `replacement_artifact_id` - ID of the new artifact replacing it.
    /// * `provenance_relation_id` - Unique ID for the auto-created PreviousVersion relation.
    pub fn supersede_artifact(
        env: Env,
        old_artifact_id: String,
        replacement_artifact_id: String,
        provenance_relation_id: String,
    ) -> Result<ArtifactRecord, RegistryError> {
        if provenance_relation_id.is_empty() {
            return Err(RegistryError::EmptyRelationId);
        }

        // Reject self-superseding
        if old_artifact_id == replacement_artifact_id {
            return Err(RegistryError::SelfReferencingNotAllowed);
        }

        // Load old artifact and authenticate its owner
        let old_key = DataKey::Artifact(old_artifact_id.clone());
        let mut old_artifact: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&old_key)
            .ok_or(RegistryError::ArtifactNotFound)?;

        old_artifact.owner.require_auth();

        if old_artifact.status == ArtifactStatus::Revoked {
            return Err(RegistryError::CannotSupersedeRevokedArtifact);
        }

        if old_artifact.status == ArtifactStatus::Superseded {
            return Err(RegistryError::ArtifactAlreadySuperseded);
        }

        // Validate replacement artifact exists and is Active
        let replacement_key = DataKey::Artifact(replacement_artifact_id.clone());
        let replacement_artifact: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&replacement_key)
            .ok_or(RegistryError::ReplacementArtifactNotFound)?;

        if replacement_artifact.status != ArtifactStatus::Active {
            return Err(RegistryError::ReplacementArtifactNotActive);
        }

        // Ensure the provenance relation ID is unique
        let rel_key = DataKey::Relation(provenance_relation_id.clone());
        if env.storage().persistent().has(&rel_key) {
            return Err(RegistryError::RelationAlreadyExists);
        }

        let timestamp = env.ledger().timestamp();

        // Mark old artifact as Superseded
        old_artifact.status = ArtifactStatus::Superseded;
        old_artifact.revoked_at = Some(timestamp);
        env.storage().persistent().set(&old_key, &old_artifact);

        // Create PreviousVersion provenance relation: replacement -> old
        let relation = ProvenanceRelation {
            relation_id: provenance_relation_id.clone(),
            source_artifact_id: replacement_artifact_id.clone(),
            target_artifact_id: old_artifact_id.clone(),
            relation_type: RelationType::PreviousVersion,
            created_at: timestamp,
        };
        env.storage().persistent().set(&rel_key, &relation);

        // Index under replacement (source)
        let src_rels_key = DataKey::ArtifactRelations(replacement_artifact_id.clone());
        let mut src_rels: Vec<String> = env
            .storage()
            .persistent()
            .get(&src_rels_key)
            .unwrap_or_else(|| Vec::new(&env));
        src_rels.push_back(provenance_relation_id.clone());
        env.storage().persistent().set(&src_rels_key, &src_rels);

        // Index under old artifact (target)
        let tgt_rels_key = DataKey::ArtifactRelations(old_artifact_id.clone());
        let mut tgt_rels: Vec<String> = env
            .storage()
            .persistent()
            .get(&tgt_rels_key)
            .unwrap_or_else(|| Vec::new(&env));
        tgt_rels.push_back(provenance_relation_id.clone());
        env.storage().persistent().set(&tgt_rels_key, &tgt_rels);

        env.events().publish(
            (
                Symbol::new(&env, "artifact_superseded"),
                old_artifact_id,
                replacement_artifact_id,
            ),
            provenance_relation_id,
        );

        Ok(old_artifact)
    }

    // -------------------------------------------------------------------------
    // Provenance Relation Functions
    // -------------------------------------------------------------------------

    /// Adds a new provenance relationship between two artifacts.
    /// Requires authentication from the owner of the source artifact.
    pub fn add_provenance_relation(
        env: Env,
        relation_id: String,
        source_artifact_id: String,
        target_artifact_id: String,
        relation_type: RelationType,
    ) -> Result<ProvenanceRelation, RegistryError> {
        if relation_id.is_empty() {
            return Err(RegistryError::EmptyRelationId);
        }

        if source_artifact_id == target_artifact_id {
            return Err(RegistryError::SelfReferencingNotAllowed);
        }

        let rel_key = DataKey::Relation(relation_id.clone());
        if env.storage().persistent().has(&rel_key) {
            return Err(RegistryError::RelationAlreadyExists);
        }

        let source_key = DataKey::Artifact(source_artifact_id.clone());
        let source_artifact: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&source_key)
            .ok_or(RegistryError::SourceArtifactNotFound)?;

        source_artifact.owner.require_auth();

        if source_artifact.status == ArtifactStatus::Revoked {
            return Err(RegistryError::SourceArtifactRevoked);
        }

        let target_key = DataKey::Artifact(target_artifact_id.clone());
        let target_artifact: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&target_key)
            .ok_or(RegistryError::TargetArtifactNotFound)?;

        if target_artifact.status == ArtifactStatus::Revoked {
            return Err(RegistryError::TargetArtifactRevoked);
        }

        let created_at = env.ledger().timestamp();

        let relation = ProvenanceRelation {
            relation_id: relation_id.clone(),
            source_artifact_id: source_artifact_id.clone(),
            target_artifact_id: target_artifact_id.clone(),
            relation_type,
            created_at,
        };

        env.storage().persistent().set(&rel_key, &relation);

        let source_rels_key = DataKey::ArtifactRelations(source_artifact_id.clone());
        let mut source_rels: Vec<String> = env
            .storage()
            .persistent()
            .get(&source_rels_key)
            .unwrap_or_else(|| Vec::new(&env));
        source_rels.push_back(relation_id.clone());
        env.storage()
            .persistent()
            .set(&source_rels_key, &source_rels);

        let target_rels_key = DataKey::ArtifactRelations(target_artifact_id.clone());
        let mut target_rels: Vec<String> = env
            .storage()
            .persistent()
            .get(&target_rels_key)
            .unwrap_or_else(|| Vec::new(&env));
        target_rels.push_back(relation_id.clone());
        env.storage()
            .persistent()
            .set(&target_rels_key, &target_rels);

        env.events().publish(
            (
                Symbol::new(&env, "provenance_relation_added"),
                source_artifact_id,
                target_artifact_id,
            ),
            relation_id,
        );

        Ok(relation)
    }

    /// Retrieves a provenance relationship record by its relation ID.
    pub fn get_provenance_relation(env: Env, relation_id: String) -> Option<ProvenanceRelation> {
        let key = DataKey::Relation(relation_id);
        env.storage().persistent().get(&key)
    }

    /// Retrieves all provenance relationships associated with a given artifact ID.
    pub fn get_artifact_relations(env: Env, artifact_id: String) -> Vec<ProvenanceRelation> {
        let rels_key = DataKey::ArtifactRelations(artifact_id);
        let rel_ids: Vec<String> = env
            .storage()
            .persistent()
            .get(&rels_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut result = Vec::new(&env);
        for id in rel_ids.iter() {
            if let Some(rel) = Self::get_provenance_relation(env.clone(), id) {
                result.push_back(rel);
            }
        }
        result
    }

    // -------------------------------------------------------------------------
    // Attestation Functions
    // -------------------------------------------------------------------------

    /// Attaches a verifiable attestation to a registered artifact.
    /// Any authorized Stellar address may attest independent of artifact ownership.
    pub fn add_attestation(
        env: Env,
        attestation_id: String,
        artifact_id: String,
        attester: Address,
        attestation_type: AttestationType,
        evidence_hash: String,
    ) -> Result<Attestation, RegistryError> {
        attester.require_auth();

        if attestation_id.is_empty() {
            return Err(RegistryError::EmptyAttestationId);
        }

        if evidence_hash.is_empty() {
            return Err(RegistryError::EmptyEvidenceHash);
        }

        let att_key = DataKey::Attestation(attestation_id.clone());
        if env.storage().persistent().has(&att_key) {
            return Err(RegistryError::AttestationAlreadyExists);
        }

        let art_key = DataKey::Artifact(artifact_id.clone());
        let artifact: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&art_key)
            .ok_or(RegistryError::ArtifactNotFound)?;

        if artifact.status == ArtifactStatus::Revoked {
            return Err(RegistryError::CannotAttestRevokedArtifact);
        }

        let created_at = env.ledger().timestamp();

        let attestation = Attestation {
            attestation_id: attestation_id.clone(),
            artifact_id: artifact_id.clone(),
            attester: attester.clone(),
            attestation_type,
            evidence_hash,
            created_at,
        };

        env.storage().persistent().set(&att_key, &attestation);

        let art_atts_key = DataKey::ArtifactAttestations(artifact_id.clone());
        let mut art_atts: Vec<String> = env
            .storage()
            .persistent()
            .get(&art_atts_key)
            .unwrap_or_else(|| Vec::new(&env));
        art_atts.push_back(attestation_id.clone());
        env.storage().persistent().set(&art_atts_key, &art_atts);

        env.events().publish(
            (
                Symbol::new(&env, "attestation_added"),
                attestation_id,
                artifact_id,
            ),
            attester,
        );

        Ok(attestation)
    }

    /// Retrieves an attestation record by its unique attestation ID.
    pub fn get_attestation(env: Env, attestation_id: String) -> Option<Attestation> {
        let key = DataKey::Attestation(attestation_id);
        env.storage().persistent().get(&key)
    }

    /// Retrieves all attestations associated with a given artifact ID.
    pub fn get_artifact_attestations(env: Env, artifact_id: String) -> Vec<Attestation> {
        let art_atts_key = DataKey::ArtifactAttestations(artifact_id);
        let att_ids: Vec<String> = env
            .storage()
            .persistent()
            .get(&art_atts_key)
            .unwrap_or_else(|| Vec::new(&env));

        let mut result = Vec::new(&env);
        for id in att_ids.iter() {
            if let Some(att) = Self::get_attestation(env.clone(), id) {
                result.push_back(att);
            }
        }
        result
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    // -------------------------------------------------------------------------
    // Helpers
    // -------------------------------------------------------------------------

    fn setup() -> (Env, ModelProofRegistryClient<'static>) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);
        (env, client)
    }

    fn register_artifact(
        env: &Env,
        client: &ModelProofRegistryClient,
        owner: &Address,
        id: &str,
    ) -> String {
        let artifact_id = String::from_str(env, id);
        client.register_artifact(
            owner,
            &artifact_id,
            &String::from_str(env, "hash-placeholder"),
            &String::from_str(env, "model"),
        );
        artifact_id
    }

    // -------------------------------------------------------------------------
    // Artifact registration tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_successful_registration() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = String::from_str(&env, "dataset-v1-001");
        let artifact_hash = String::from_str(
            &env,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        );
        let artifact_type = String::from_str(&env, "dataset");

        let record = client.register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type);

        assert_eq!(record.artifact_id, artifact_id);
        assert_eq!(record.artifact_hash, artifact_hash);
        assert_eq!(record.artifact_type, artifact_type);
        assert_eq!(record.owner, owner);
    }

    #[test]
    fn test_new_artifact_defaults_to_active() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "art-default-status");

        let record = client.get_artifact(&artifact_id).unwrap();
        assert_eq!(record.status, ArtifactStatus::Active);
        assert!(record.revoked_at.is_none());
        assert!(record.revocation_reason_hash.is_none());
    }

    #[test]
    fn test_artifact_retrieval() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = String::from_str(&env, "model-resnet50");
        let artifact_hash = String::from_str(&env, "8f48937...hash");
        let artifact_type = String::from_str(&env, "model");

        assert!(client.get_artifact(&artifact_id).is_none());
        client.register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type);

        let record = client.get_artifact(&artifact_id).unwrap();
        assert_eq!(record.artifact_id, artifact_id);
        assert_eq!(record.artifact_hash, artifact_hash);
        assert_eq!(record.artifact_type, artifact_type);
        assert_eq!(record.owner, owner);
    }

    #[test]
    fn test_duplicate_registration_rejection() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = String::from_str(&env, "eval-run-42");
        let artifact_hash = String::from_str(&env, "hash-eval-42");
        let artifact_type = String::from_str(&env, "evaluation");

        assert!(client
            .try_register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type)
            .is_ok());
        assert_eq!(
            client.try_register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type),
            Err(Ok(RegistryError::AlreadyExists))
        );
    }

    // -------------------------------------------------------------------------
    // Provenance relation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_successful_provenance_relation_creation() {
        let (env, client) = setup();
        let owner_src = Address::generate(&env);
        let owner_tgt = Address::generate(&env);
        let src_id = String::from_str(&env, "model-gpt-fine-tune");
        let tgt_id = String::from_str(&env, "dataset-openwebtext");

        client.register_artifact(
            &owner_src,
            &src_id,
            &String::from_str(&env, "hash-model"),
            &String::from_str(&env, "model"),
        );
        client.register_artifact(
            &owner_tgt,
            &tgt_id,
            &String::from_str(&env, "hash-dataset"),
            &String::from_str(&env, "dataset"),
        );

        let rel_id = String::from_str(&env, "rel-trained-on-001");
        let relation =
            client.add_provenance_relation(&rel_id, &src_id, &tgt_id, &RelationType::TrainedOn);

        assert_eq!(relation.relation_id, rel_id);
        assert_eq!(relation.source_artifact_id, src_id);
        assert_eq!(relation.target_artifact_id, tgt_id);
        assert_eq!(relation.relation_type, RelationType::TrainedOn);
    }

    #[test]
    fn test_get_provenance_relation() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let src_id = String::from_str(&env, "model-v2");
        let tgt_id = String::from_str(&env, "model-v1");
        let rel_id = String::from_str(&env, "rel-prev-version-100");

        assert!(client.get_provenance_relation(&rel_id).is_none());

        client.register_artifact(
            &owner,
            &src_id,
            &String::from_str(&env, "hash-v2"),
            &String::from_str(&env, "model_version"),
        );
        client.register_artifact(
            &owner,
            &tgt_id,
            &String::from_str(&env, "hash-v1"),
            &String::from_str(&env, "model_version"),
        );
        client.add_provenance_relation(&rel_id, &src_id, &tgt_id, &RelationType::PreviousVersion);

        let rel = client.get_provenance_relation(&rel_id).unwrap();
        assert_eq!(rel.relation_id, rel_id);
        assert_eq!(rel.source_artifact_id, src_id);
        assert_eq!(rel.target_artifact_id, tgt_id);
        assert_eq!(rel.relation_type, RelationType::PreviousVersion);
    }

    #[test]
    fn test_get_artifact_relations() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let art_a = String::from_str(&env, "art-A");
        let art_b = String::from_str(&env, "art-B");
        let art_c = String::from_str(&env, "art-C");

        for (id, hash, typ) in [
            (&art_a, "hA", "typeA"),
            (&art_b, "hB", "typeB"),
            (&art_c, "hC", "typeC"),
        ] {
            client.register_artifact(
                &owner,
                id,
                &String::from_str(&env, hash),
                &String::from_str(&env, typ),
            );
        }

        let rel1_id = String::from_str(&env, "rel-1");
        let rel2_id = String::from_str(&env, "rel-2");
        client.add_provenance_relation(&rel1_id, &art_a, &art_b, &RelationType::DerivedFrom);
        client.add_provenance_relation(&rel2_id, &art_a, &art_c, &RelationType::EvaluatedWith);

        assert_eq!(client.get_artifact_relations(&art_a).len(), 2);
        let rels_b = client.get_artifact_relations(&art_b);
        assert_eq!(rels_b.len(), 1);
        assert_eq!(rels_b.get(0).unwrap().relation_id, rel1_id);
        let rels_c = client.get_artifact_relations(&art_c);
        assert_eq!(rels_c.len(), 1);
        assert_eq!(rels_c.get(0).unwrap().relation_id, rel2_id);
    }

    #[test]
    fn test_reject_duplicate_relation_id() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let src_id = register_artifact(&env, &client, &owner, "art-source");
        let tgt1_id = register_artifact(&env, &client, &owner, "art-target-1");
        let tgt2_id = register_artifact(&env, &client, &owner, "art-target-2");
        let rel_id = String::from_str(&env, "rel-dup-id");

        assert!(client
            .try_add_provenance_relation(&rel_id, &src_id, &tgt1_id, &RelationType::ProducedBy)
            .is_ok());
        assert_eq!(
            client.try_add_provenance_relation(
                &rel_id,
                &src_id,
                &tgt2_id,
                &RelationType::ProducedBy
            ),
            Err(Ok(RegistryError::RelationAlreadyExists))
        );
    }

    #[test]
    fn test_reject_unknown_source_artifact() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let valid_tgt = register_artifact(&env, &client, &owner, "valid-target");

        assert_eq!(
            client.try_add_provenance_relation(
                &String::from_str(&env, "rel-err-1"),
                &String::from_str(&env, "unknown-source"),
                &valid_tgt,
                &RelationType::DerivedFrom,
            ),
            Err(Ok(RegistryError::SourceArtifactNotFound))
        );
    }

    #[test]
    fn test_reject_unknown_target_artifact() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let valid_src = register_artifact(&env, &client, &owner, "valid-source");

        assert_eq!(
            client.try_add_provenance_relation(
                &String::from_str(&env, "rel-err-2"),
                &valid_src,
                &String::from_str(&env, "unknown-target"),
                &RelationType::TrainedOn,
            ),
            Err(Ok(RegistryError::TargetArtifactNotFound))
        );
    }

    #[test]
    fn test_reject_self_referencing_relation() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let art_self = register_artifact(&env, &client, &owner, "art-self-ref");

        assert_eq!(
            client.try_add_provenance_relation(
                &String::from_str(&env, "rel-self-1"),
                &art_self,
                &art_self,
                &RelationType::PreviousVersion,
            ),
            Err(Ok(RegistryError::SelfReferencingNotAllowed))
        );
    }

    // -------------------------------------------------------------------------
    // Attestation tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_successful_attestation_creation() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "model-att-1");
        let att_id = String::from_str(&env, "att-001");
        let evidence = String::from_str(&env, "sha256-abc123");

        let att = client.add_attestation(
            &att_id,
            &artifact_id,
            &attester,
            &AttestationType::Verified,
            &evidence,
        );

        assert_eq!(att.attestation_id, att_id);
        assert_eq!(att.artifact_id, artifact_id);
        assert_eq!(att.attester, attester);
        assert_eq!(att.attestation_type, AttestationType::Verified);
        assert_eq!(att.evidence_hash, evidence);
    }

    #[test]
    fn test_attestation_retrieval() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "model-att-2");
        let att_id = String::from_str(&env, "att-002");

        assert!(client.get_attestation(&att_id).is_none());
        client.add_attestation(
            &att_id,
            &artifact_id,
            &attester,
            &AttestationType::Audited,
            &String::from_str(&env, "sha256-evidence"),
        );

        let att = client.get_attestation(&att_id).unwrap();
        assert_eq!(att.attestation_id, att_id);
        assert_eq!(att.artifact_id, artifact_id);
        assert_eq!(att.attestation_type, AttestationType::Audited);
    }

    #[test]
    fn test_artifact_attestation_listing() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester_a = Address::generate(&env);
        let attester_b = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "model-att-list");
        let att_id_1 = String::from_str(&env, "att-list-001");
        let att_id_2 = String::from_str(&env, "att-list-002");

        assert_eq!(client.get_artifact_attestations(&artifact_id).len(), 0);

        client.add_attestation(
            &att_id_1,
            &artifact_id,
            &attester_a,
            &AttestationType::Verified,
            &String::from_str(&env, "ev-hash-1"),
        );
        client.add_attestation(
            &att_id_2,
            &artifact_id,
            &attester_b,
            &AttestationType::Reproduced,
            &String::from_str(&env, "ev-hash-2"),
        );

        let atts = client.get_artifact_attestations(&artifact_id);
        assert_eq!(atts.len(), 2);
        assert_eq!(atts.get(0).unwrap().attestation_id, att_id_1);
        assert_eq!(atts.get(1).unwrap().attestation_id, att_id_2);
    }

    #[test]
    fn test_reject_duplicate_attestation_id() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "model-att-dup");
        let att_id = String::from_str(&env, "att-dup-001");
        let evidence = String::from_str(&env, "ev-hash");

        assert!(client
            .try_add_attestation(
                &att_id,
                &artifact_id,
                &attester,
                &AttestationType::Endorsed,
                &evidence,
            )
            .is_ok());
        assert_eq!(
            client.try_add_attestation(
                &att_id,
                &artifact_id,
                &attester,
                &AttestationType::Endorsed,
                &evidence,
            ),
            Err(Ok(RegistryError::AttestationAlreadyExists))
        );
    }

    #[test]
    fn test_reject_attestation_for_unknown_artifact() {
        let (env, client) = setup();
        let attester = Address::generate(&env);

        assert_eq!(
            client.try_add_attestation(
                &String::from_str(&env, "att-unknown"),
                &String::from_str(&env, "nonexistent-artifact"),
                &attester,
                &AttestationType::Evaluated,
                &String::from_str(&env, "ev-hash"),
            ),
            Err(Ok(RegistryError::ArtifactNotFound))
        );
    }

    #[test]
    fn test_attester_authentication_required() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "model-att-auth");
        let att_id = String::from_str(&env, "att-auth-001");

        let res = client.try_add_attestation(
            &att_id,
            &artifact_id,
            &attester,
            &AttestationType::Verified,
            &String::from_str(&env, "ev-hash-auth"),
        );
        // mock_all_auths satisfies the require_auth call; success confirms auth was invoked
        assert!(res.is_ok());

        // Verify attester's auth was recorded
        let recorded_auths = env.auths();
        let attester_authed = recorded_auths.iter().any(|(addr, _)| *addr == attester);
        assert!(attester_authed, "attester.require_auth() must be recorded");
    }

    #[test]
    fn test_reject_empty_evidence_hash() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "model-att-empty-ev");

        assert_eq!(
            client.try_add_attestation(
                &String::from_str(&env, "att-empty-ev"),
                &artifact_id,
                &attester,
                &AttestationType::Verified,
                &String::from_str(&env, ""),
            ),
            Err(Ok(RegistryError::EmptyEvidenceHash))
        );
    }

    // -------------------------------------------------------------------------
    // Artifact lifecycle tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_successful_revocation() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "art-revoke-1");
        let reason_hash = String::from_str(&env, "sha256-revocation-reason");

        let record = client.revoke_artifact(&artifact_id, &reason_hash);

        assert_eq!(record.status, ArtifactStatus::Revoked);
        assert!(record.revoked_at.is_some());
        assert_eq!(record.revocation_reason_hash.unwrap(), reason_hash);

        // Persisted correctly
        let stored = client.get_artifact(&artifact_id).unwrap();
        assert_eq!(stored.status, ArtifactStatus::Revoked);
    }

    #[test]
    fn test_revocation_owner_auth_recorded() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "art-revoke-auth");

        client.revoke_artifact(&artifact_id, &String::from_str(&env, "sha256-reason"));

        // Verify that the owner's auth was invoked
        let recorded = env.auths();
        let owner_authed = recorded.iter().any(|(addr, _)| *addr == owner);
        assert!(
            owner_authed,
            "owner.require_auth() must be recorded during revocation"
        );
    }

    #[test]
    fn test_reject_revoke_unknown_artifact() {
        let (env, client) = setup();

        assert_eq!(
            client.try_revoke_artifact(
                &String::from_str(&env, "nonexistent-art"),
                &String::from_str(&env, "sha256-reason"),
            ),
            Err(Ok(RegistryError::ArtifactNotFound))
        );
    }

    #[test]
    fn test_reject_duplicate_revocation() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "art-revoke-dup");
        let reason = String::from_str(&env, "sha256-reason");

        assert!(client.try_revoke_artifact(&artifact_id, &reason).is_ok());
        assert_eq!(
            client.try_revoke_artifact(&artifact_id, &reason),
            Err(Ok(RegistryError::ArtifactAlreadyRevoked))
        );
    }

    #[test]
    fn test_reject_empty_revocation_reason() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let artifact_id = register_artifact(&env, &client, &owner, "art-revoke-empty");

        assert_eq!(
            client.try_revoke_artifact(&artifact_id, &String::from_str(&env, "")),
            Err(Ok(RegistryError::EmptyRevocationReason))
        );
    }

    #[test]
    fn test_successful_superseding() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "model-v1");
        let new_id = register_artifact(&env, &client, &owner, "model-v2");
        let rel_id = String::from_str(&env, "supersede-rel-001");

        let old_record = client.supersede_artifact(&old_id, &new_id, &rel_id);

        assert_eq!(old_record.status, ArtifactStatus::Superseded);
        assert!(old_record.revoked_at.is_some());

        // PreviousVersion relation created: new -> old
        let rel = client.get_provenance_relation(&rel_id).unwrap();
        assert_eq!(rel.source_artifact_id, new_id);
        assert_eq!(rel.target_artifact_id, old_id);
        assert_eq!(rel.relation_type, RelationType::PreviousVersion);

        // Relation indexed under both artifacts
        let old_rels = client.get_artifact_relations(&old_id);
        assert_eq!(old_rels.len(), 1);
        let new_rels = client.get_artifact_relations(&new_id);
        assert_eq!(new_rels.len(), 1);
    }

    #[test]
    fn test_reject_self_superseding() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let art_id = register_artifact(&env, &client, &owner, "art-self-sup");

        assert_eq!(
            client.try_supersede_artifact(&art_id, &art_id, &String::from_str(&env, "rel-self"),),
            Err(Ok(RegistryError::SelfReferencingNotAllowed))
        );
    }

    #[test]
    fn test_reject_supersede_unknown_replacement() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "art-sup-old");

        assert_eq!(
            client.try_supersede_artifact(
                &old_id,
                &String::from_str(&env, "nonexistent-replacement"),
                &String::from_str(&env, "rel-sup-err"),
            ),
            Err(Ok(RegistryError::ReplacementArtifactNotFound))
        );
    }

    #[test]
    fn test_provenance_and_attestations_remain_after_lifecycle_changes() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);

        // Register two artifacts with a provenance relation between them
        let art_a = register_artifact(&env, &client, &owner, "art-lifecycle-a");
        let art_b = register_artifact(&env, &client, &owner, "art-lifecycle-b");

        let rel_id = String::from_str(&env, "rel-lifecycle-001");
        client.add_provenance_relation(&rel_id, &art_a, &art_b, &RelationType::DerivedFrom);

        // Attest to artifact A
        let att_id = String::from_str(&env, "att-lifecycle-001");
        client.add_attestation(
            &att_id,
            &art_a,
            &attester,
            &AttestationType::Verified,
            &String::from_str(&env, "sha256-lifecycle-ev"),
        );

        // Revoke artifact A
        client.revoke_artifact(&art_a, &String::from_str(&env, "sha256-revoke-reason"));

        // Provenance relation is still queryable
        let rel = client.get_provenance_relation(&rel_id);
        assert!(rel.is_some());
        assert_eq!(rel.unwrap().relation_id, rel_id);

        // Attestation is still queryable
        let att = client.get_attestation(&att_id);
        assert!(att.is_some());
        assert_eq!(att.unwrap().attestation_id, att_id);

        // Artifact A is retrievable (status = Revoked, history preserved)
        let record = client.get_artifact(&art_a).unwrap();
        assert_eq!(record.status, ArtifactStatus::Revoked);

        // Artifact B is unaffected
        let record_b = client.get_artifact(&art_b).unwrap();
        assert_eq!(record_b.status, ArtifactStatus::Active);
    }

    // -------------------------------------------------------------------------
    // Input validation and security invariant tests
    // -------------------------------------------------------------------------

    #[test]
    fn test_reject_empty_artifact_id() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let res = client.try_register_artifact(
            &owner,
            &String::from_str(&env, ""),
            &String::from_str(&env, "hash-001"),
            &String::from_str(&env, "dataset"),
        );
        assert_eq!(res, Err(Ok(RegistryError::EmptyArtifactId)));
    }

    #[test]
    fn test_reject_empty_artifact_hash() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let res = client.try_register_artifact(
            &owner,
            &String::from_str(&env, "art-valid-id"),
            &String::from_str(&env, ""),
            &String::from_str(&env, "dataset"),
        );
        assert_eq!(res, Err(Ok(RegistryError::EmptyArtifactHash)));
    }

    #[test]
    fn test_reject_empty_artifact_type() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let res = client.try_register_artifact(
            &owner,
            &String::from_str(&env, "art-valid-id"),
            &String::from_str(&env, "hash-001"),
            &String::from_str(&env, ""),
        );
        assert_eq!(res, Err(Ok(RegistryError::EmptyArtifactType)));
    }

    #[test]
    fn test_reject_empty_provenance_relation_id() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let src_id = register_artifact(&env, &client, &owner, "art-src-empty-rel");
        let tgt_id = register_artifact(&env, &client, &owner, "art-tgt-empty-rel");

        let res = client.try_add_provenance_relation(
            &String::from_str(&env, ""),
            &src_id,
            &tgt_id,
            &RelationType::DerivedFrom,
        );
        assert_eq!(res, Err(Ok(RegistryError::EmptyRelationId)));

        let res_sup = client.try_supersede_artifact(&src_id, &tgt_id, &String::from_str(&env, ""));
        assert_eq!(res_sup, Err(Ok(RegistryError::EmptyRelationId)));
    }

    #[test]
    fn test_reject_empty_attestation_id() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let art_id = register_artifact(&env, &client, &owner, "art-empty-att");

        let res = client.try_add_attestation(
            &String::from_str(&env, ""),
            &art_id,
            &attester,
            &AttestationType::Verified,
            &String::from_str(&env, "sha256-evidence"),
        );
        assert_eq!(res, Err(Ok(RegistryError::EmptyAttestationId)));
    }

    #[test]
    fn test_reject_provenance_relation_using_revoked_source() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let src_id = register_artifact(&env, &client, &owner, "art-src-revoked");
        let tgt_id = register_artifact(&env, &client, &owner, "art-tgt-active");

        client.revoke_artifact(&src_id, &String::from_str(&env, "revocation-reason"));

        let res = client.try_add_provenance_relation(
            &String::from_str(&env, "rel-rev-src"),
            &src_id,
            &tgt_id,
            &RelationType::DerivedFrom,
        );
        assert_eq!(res, Err(Ok(RegistryError::SourceArtifactRevoked)));
    }

    #[test]
    fn test_reject_provenance_relation_using_revoked_target() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let src_id = register_artifact(&env, &client, &owner, "art-src-active");
        let tgt_id = register_artifact(&env, &client, &owner, "art-tgt-revoked");

        client.revoke_artifact(&tgt_id, &String::from_str(&env, "revocation-reason"));

        let res = client.try_add_provenance_relation(
            &String::from_str(&env, "rel-rev-tgt"),
            &src_id,
            &tgt_id,
            &RelationType::DerivedFrom,
        );
        assert_eq!(res, Err(Ok(RegistryError::TargetArtifactRevoked)));
    }

    #[test]
    fn test_reject_attestation_on_revoked_artifact() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);
        let art_id = register_artifact(&env, &client, &owner, "art-att-revoked");

        client.revoke_artifact(&art_id, &String::from_str(&env, "revocation-reason"));

        let res = client.try_add_attestation(
            &String::from_str(&env, "att-on-revoked"),
            &art_id,
            &attester,
            &AttestationType::Audited,
            &String::from_str(&env, "sha256-evidence"),
        );
        assert_eq!(res, Err(Ok(RegistryError::CannotAttestRevokedArtifact)));
    }

    #[test]
    fn test_reject_revoking_superseded_artifact() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "art-old-to-sup");
        let new_id = register_artifact(&env, &client, &owner, "art-new-rep");

        client.supersede_artifact(&old_id, &new_id, &String::from_str(&env, "rel-sup-ok"));

        let res = client.try_revoke_artifact(&old_id, &String::from_str(&env, "sha256-reason"));
        assert_eq!(res, Err(Ok(RegistryError::CannotRevokeSupersededArtifact)));
    }

    #[test]
    fn test_reject_superseding_revoked_artifact() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "art-old-rev");
        let new_id = register_artifact(&env, &client, &owner, "art-new-act");

        client.revoke_artifact(&old_id, &String::from_str(&env, "sha256-reason"));

        let res =
            client.try_supersede_artifact(&old_id, &new_id, &String::from_str(&env, "rel-sup-rev"));
        assert_eq!(res, Err(Ok(RegistryError::CannotSupersedeRevokedArtifact)));
    }

    #[test]
    fn test_reject_superseding_already_superseded_artifact() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "art-old-v1");
        let rep_v2 = register_artifact(&env, &client, &owner, "art-rep-v2");
        let rep_v3 = register_artifact(&env, &client, &owner, "art-rep-v3");

        client.supersede_artifact(&old_id, &rep_v2, &String::from_str(&env, "rel-v1-to-v2"));

        let res = client.try_supersede_artifact(
            &old_id,
            &rep_v3,
            &String::from_str(&env, "rel-v1-to-v3"),
        );
        assert_eq!(res, Err(Ok(RegistryError::ArtifactAlreadySuperseded)));
    }

    #[test]
    fn test_reject_using_revoked_artifact_as_replacement() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "art-to-replace");
        let revoked_rep = register_artifact(&env, &client, &owner, "art-revoked-rep");

        client.revoke_artifact(&revoked_rep, &String::from_str(&env, "sha256-reason"));

        let res = client.try_supersede_artifact(
            &old_id,
            &revoked_rep,
            &String::from_str(&env, "rel-rev-rep"),
        );
        assert_eq!(res, Err(Ok(RegistryError::ReplacementArtifactNotActive)));
    }

    #[test]
    fn test_reject_using_superseded_artifact_as_replacement() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let old_id = register_artifact(&env, &client, &owner, "art-current");
        let mid_id = register_artifact(&env, &client, &owner, "art-mid");
        let latest_id = register_artifact(&env, &client, &owner, "art-latest");

        // Supersede mid_id with latest_id, making mid_id Superseded
        client.supersede_artifact(
            &mid_id,
            &latest_id,
            &String::from_str(&env, "rel-mid-latest"),
        );

        // Attempt to use superseded mid_id as a replacement for old_id
        let res = client.try_supersede_artifact(
            &old_id,
            &mid_id,
            &String::from_str(&env, "rel-use-superseded-rep"),
        );
        assert_eq!(res, Err(Ok(RegistryError::ReplacementArtifactNotActive)));
    }

    #[test]
    fn test_storage_remains_unchanged_after_rejected_operations() {
        let (env, client) = setup();
        let owner = Address::generate(&env);
        let attester = Address::generate(&env);

        let art_id = register_artifact(&env, &client, &owner, "art-storage-test");
        let initial_record = client.get_artifact(&art_id).unwrap();
        assert_eq!(initial_record.status, ArtifactStatus::Active);

        // 1. Rejected registration with empty id: storage has no empty key
        assert_eq!(
            client.try_register_artifact(
                &owner,
                &String::from_str(&env, ""),
                &String::from_str(&env, "hash"),
                &String::from_str(&env, "model"),
            ),
            Err(Ok(RegistryError::EmptyArtifactId))
        );
        assert!(client.get_artifact(&String::from_str(&env, "")).is_none());

        // 2. Rejected relation with empty relation ID: no relation stored, relations list untouched
        assert_eq!(
            client.try_add_provenance_relation(
                &String::from_str(&env, ""),
                &art_id,
                &art_id,
                &RelationType::DerivedFrom,
            ),
            Err(Ok(RegistryError::EmptyRelationId))
        );
        assert_eq!(client.get_artifact_relations(&art_id).len(), 0);

        // 3. Rejected attestation with empty attestation ID: no attestation stored, list untouched
        assert_eq!(
            client.try_add_attestation(
                &String::from_str(&env, ""),
                &art_id,
                &attester,
                &AttestationType::Verified,
                &String::from_str(&env, "sha256-ev"),
            ),
            Err(Ok(RegistryError::EmptyAttestationId))
        );
        assert_eq!(client.get_artifact_attestations(&art_id).len(), 0);

        // 4. Rejected superseding with non-active replacement: old artifact remains Active, no relation
        let rep_revoked = register_artifact(&env, &client, &owner, "art-rep-rev-storage");
        client.revoke_artifact(&rep_revoked, &String::from_str(&env, "reason"));
        let bad_rel_id = String::from_str(&env, "rel-storage-rejected");

        assert_eq!(
            client.try_supersede_artifact(&art_id, &rep_revoked, &bad_rel_id),
            Err(Ok(RegistryError::ReplacementArtifactNotActive))
        );
        let record_after_failed_supersede = client.get_artifact(&art_id).unwrap();
        assert_eq!(record_after_failed_supersede.status, ArtifactStatus::Active);
        assert!(record_after_failed_supersede.revoked_at.is_none());
        assert!(client.get_provenance_relation(&bad_rel_id).is_none());
        assert_eq!(client.get_artifact_relations(&art_id).len(), 0);

        // 5. Rejected revocation of superseded artifact: status and reason unchanged
        let new_valid_rep = register_artifact(&env, &client, &owner, "art-valid-rep");
        let valid_rel_id = String::from_str(&env, "rel-valid-sup");
        client.supersede_artifact(&art_id, &new_valid_rep, &valid_rel_id);

        let superseded_record = client.get_artifact(&art_id).unwrap();
        assert_eq!(superseded_record.status, ArtifactStatus::Superseded);

        assert_eq!(
            client.try_revoke_artifact(&art_id, &String::from_str(&env, "new-attempted-reason")),
            Err(Ok(RegistryError::CannotRevokeSupersededArtifact))
        );
        let final_record = client.get_artifact(&art_id).unwrap();
        assert_eq!(final_record.status, ArtifactStatus::Superseded);
        assert_eq!(final_record.revocation_reason_hash, None);
    }
}
