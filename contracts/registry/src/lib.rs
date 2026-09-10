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

/// Storage keys for the contract state.
#[contracttype]
pub enum DataKey {
    Artifact(String),
    Relation(String),
    ArtifactRelations(String),
}

/// Smart contract for registering and verifying AI artifact records and provenance relationships.
#[contract]
pub struct ModelProofRegistry;

#[contractimpl]
impl ModelProofRegistry {
    /// Registers a new AI artifact in the provenance registry.
    ///
    /// # Arguments
    /// * `env` - Soroban environment handle.
    /// * `owner` - Address of the entity registering the artifact.
    /// * `artifact_id` - Unique identifier for the artifact.
    /// * `artifact_hash` - Cryptographic hash of the artifact content/metadata.
    /// * `artifact_type` - Type category (e.g. dataset, model, training_run, evaluation).
    ///
    /// # Returns
    /// * `Ok(ArtifactRecord)` if successfully registered.
    /// * `Err(RegistryError::AlreadyExists)` if the artifact ID is already registered.
    pub fn register_artifact(
        env: Env,
        owner: Address,
        artifact_id: String,
        artifact_hash: String,
        artifact_type: String,
    ) -> Result<ArtifactRecord, RegistryError> {
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

    /// Adds a new provenance relationship between two artifacts.
    /// Requires authentication from the owner of the source artifact.
    ///
    /// # Arguments
    /// * `env` - Soroban environment handle.
    /// * `relation_id` - Unique identifier for the provenance relationship.
    /// * `source_artifact_id` - ID of the originating artifact.
    /// * `target_artifact_id` - ID of the target/referenced artifact.
    /// * `relation_type` - Type of relationship (e.g. DerivedFrom, TrainedOn, etc.).
    ///
    /// # Returns
    /// * `Ok(ProvenanceRelation)` if successfully created.
    /// * `Err(RegistryError)` on validation failure.
    pub fn add_provenance_relation(
        env: Env,
        relation_id: String,
        source_artifact_id: String,
        target_artifact_id: String,
        relation_type: RelationType,
    ) -> Result<ProvenanceRelation, RegistryError> {
        // Prevent self-referencing
        if source_artifact_id == target_artifact_id {
            return Err(RegistryError::SelfReferencingNotAllowed);
        }

        // Relation ID uniqueness check
        let rel_key = DataKey::Relation(relation_id.clone());
        if env.storage().persistent().has(&rel_key) {
            return Err(RegistryError::RelationAlreadyExists);
        }

        // Validate source artifact exists and authenticate its owner
        let source_key = DataKey::Artifact(source_artifact_id.clone());
        let source_artifact: ArtifactRecord = env
            .storage()
            .persistent()
            .get(&source_key)
            .ok_or(RegistryError::SourceArtifactNotFound)?;

        source_artifact.owner.require_auth();

        // Validate target artifact exists
        let target_key = DataKey::Artifact(target_artifact_id.clone());
        if !env.storage().persistent().has(&target_key) {
            return Err(RegistryError::TargetArtifactNotFound);
        }

        let created_at = env.ledger().timestamp();

        let relation = ProvenanceRelation {
            relation_id: relation_id.clone(),
            source_artifact_id: source_artifact_id.clone(),
            target_artifact_id: target_artifact_id.clone(),
            relation_type,
            created_at,
        };

        // Persist relation
        env.storage().persistent().set(&rel_key, &relation);

        // Index relationship under source artifact
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

        // Index relationship under target artifact
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

        // Emit provenance_relation_added event
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
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env, String};

    #[test]
    fn test_successful_registration() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

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
    fn test_artifact_retrieval() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let artifact_id = String::from_str(&env, "model-resnet50");
        let artifact_hash = String::from_str(&env, "8f48937...hash");
        let artifact_type = String::from_str(&env, "model");

        // Prior to registration, should be None
        let not_found = client.get_artifact(&artifact_id);
        assert!(not_found.is_none());

        // Register artifact
        client.register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type);

        // After registration, retrieval returns exact record
        let retrieved = client.get_artifact(&artifact_id);
        assert!(retrieved.is_some());
        let record = retrieved.unwrap();
        assert_eq!(record.artifact_id, artifact_id);
        assert_eq!(record.artifact_hash, artifact_hash);
        assert_eq!(record.artifact_type, artifact_type);
        assert_eq!(record.owner, owner);
    }

    #[test]
    fn test_duplicate_registration_rejection() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let artifact_id = String::from_str(&env, "eval-run-42");
        let artifact_hash = String::from_str(&env, "hash-eval-42");
        let artifact_type = String::from_str(&env, "evaluation");

        // First registration succeeds
        let res1 =
            client.try_register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type);
        assert!(res1.is_ok());

        // Second registration with duplicate ID fails
        let res2 =
            client.try_register_artifact(&owner, &artifact_id, &artifact_hash, &artifact_type);
        assert_eq!(res2, Err(Ok(RegistryError::AlreadyExists)));
    }

    #[test]
    fn test_successful_provenance_relation_creation() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

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
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let src_id = String::from_str(&env, "model-v2");
        let tgt_id = String::from_str(&env, "model-v1");
        let rel_id = String::from_str(&env, "rel-prev-version-100");

        // Lookup before creation returns None
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

        let retrieved = client.get_provenance_relation(&rel_id);
        assert!(retrieved.is_some());
        let rel = retrieved.unwrap();
        assert_eq!(rel.relation_id, rel_id);
        assert_eq!(rel.source_artifact_id, src_id);
        assert_eq!(rel.target_artifact_id, tgt_id);
        assert_eq!(rel.relation_type, RelationType::PreviousVersion);
    }

    #[test]
    fn test_get_artifact_relations() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let art_a = String::from_str(&env, "art-A");
        let art_b = String::from_str(&env, "art-B");
        let art_c = String::from_str(&env, "art-C");

        client.register_artifact(
            &owner,
            &art_a,
            &String::from_str(&env, "hA"),
            &String::from_str(&env, "typeA"),
        );
        client.register_artifact(
            &owner,
            &art_b,
            &String::from_str(&env, "hB"),
            &String::from_str(&env, "typeB"),
        );
        client.register_artifact(
            &owner,
            &art_c,
            &String::from_str(&env, "hC"),
            &String::from_str(&env, "typeC"),
        );

        let rel1_id = String::from_str(&env, "rel-1");
        let rel2_id = String::from_str(&env, "rel-2");

        client.add_provenance_relation(&rel1_id, &art_a, &art_b, &RelationType::DerivedFrom);
        client.add_provenance_relation(&rel2_id, &art_a, &art_c, &RelationType::EvaluatedWith);

        let rels_a = client.get_artifact_relations(&art_a);
        assert_eq!(rels_a.len(), 2);

        let rels_b = client.get_artifact_relations(&art_b);
        assert_eq!(rels_b.len(), 1);
        assert_eq!(rels_b.get(0).unwrap().relation_id, rel1_id);

        let rels_c = client.get_artifact_relations(&art_c);
        assert_eq!(rels_c.len(), 1);
        assert_eq!(rels_c.get(0).unwrap().relation_id, rel2_id);
    }

    #[test]
    fn test_reject_duplicate_relation_id() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let src_id = String::from_str(&env, "art-source");
        let tgt1_id = String::from_str(&env, "art-target-1");
        let tgt2_id = String::from_str(&env, "art-target-2");
        let rel_id = String::from_str(&env, "rel-dup-id");

        client.register_artifact(
            &owner,
            &src_id,
            &String::from_str(&env, "hS"),
            &String::from_str(&env, "typeS"),
        );
        client.register_artifact(
            &owner,
            &tgt1_id,
            &String::from_str(&env, "hT1"),
            &String::from_str(&env, "typeT"),
        );
        client.register_artifact(
            &owner,
            &tgt2_id,
            &String::from_str(&env, "hT2"),
            &String::from_str(&env, "typeT"),
        );

        // First relation creation succeeds
        let res1 = client.try_add_provenance_relation(
            &rel_id,
            &src_id,
            &tgt1_id,
            &RelationType::ProducedBy,
        );
        assert!(res1.is_ok());

        // Second creation with duplicate relation ID fails
        let res2 = client.try_add_provenance_relation(
            &rel_id,
            &src_id,
            &tgt2_id,
            &RelationType::ProducedBy,
        );
        assert_eq!(res2, Err(Ok(RegistryError::RelationAlreadyExists)));
    }

    #[test]
    fn test_reject_unknown_source_artifact() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let unknown_src = String::from_str(&env, "unknown-source");
        let valid_tgt = String::from_str(&env, "valid-target");

        client.register_artifact(
            &owner,
            &valid_tgt,
            &String::from_str(&env, "hT"),
            &String::from_str(&env, "typeT"),
        );

        let res = client.try_add_provenance_relation(
            &String::from_str(&env, "rel-err-1"),
            &unknown_src,
            &valid_tgt,
            &RelationType::DerivedFrom,
        );

        assert_eq!(res, Err(Ok(RegistryError::SourceArtifactNotFound)));
    }

    #[test]
    fn test_reject_unknown_target_artifact() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let valid_src = String::from_str(&env, "valid-source");
        let unknown_tgt = String::from_str(&env, "unknown-target");

        client.register_artifact(
            &owner,
            &valid_src,
            &String::from_str(&env, "hS"),
            &String::from_str(&env, "typeS"),
        );

        let res = client.try_add_provenance_relation(
            &String::from_str(&env, "rel-err-2"),
            &valid_src,
            &unknown_tgt,
            &RelationType::TrainedOn,
        );

        assert_eq!(res, Err(Ok(RegistryError::TargetArtifactNotFound)));
    }

    #[test]
    fn test_reject_self_referencing_relation() {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(ModelProofRegistry, ());
        let client = ModelProofRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let art_self = String::from_str(&env, "art-self-ref");

        client.register_artifact(
            &owner,
            &art_self,
            &String::from_str(&env, "hSelf"),
            &String::from_str(&env, "typeSelf"),
        );

        let res = client.try_add_provenance_relation(
            &String::from_str(&env, "rel-self-1"),
            &art_self,
            &art_self,
            &RelationType::PreviousVersion,
        );

        assert_eq!(res, Err(Ok(RegistryError::SelfReferencingNotAllowed)));
    }
}
