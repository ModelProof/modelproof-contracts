#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, String, Symbol,
};

/// Error types returned by the ModelProof registry contract.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RegistryError {
    AlreadyExists = 1,
    NotFound = 2,
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

/// Storage keys for the contract state.
#[contracttype]
pub enum DataKey {
    Artifact(String),
}

/// Smart contract for registering and verifying AI artifact records.
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
    ///
    /// # Arguments
    /// * `env` - Soroban environment handle.
    /// * `artifact_id` - Unique identifier for the artifact to look up.
    ///
    /// # Returns
    /// * `Some(ArtifactRecord)` if found.
    /// * `None` if no record exists for the given ID.
    pub fn get_artifact(env: Env, artifact_id: String) -> Option<ArtifactRecord> {
        let key = DataKey::Artifact(artifact_id);
        env.storage().persistent().get(&key)
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
}
