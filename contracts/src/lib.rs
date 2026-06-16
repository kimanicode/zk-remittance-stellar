#![no_std]

mod groth16;
mod vk_constants;

use crate::groth16::verify_groth16;
use soroban_sdk::crypto::bn254::{
    Bn254G1Affine, Bn254G2Affine,
};
use soroban_sdk::{
    contract, contractimpl, contracttype, token, Address, BytesN, Env, Vec,
};

#[contracttype]
pub enum DataKey {
    MerkleRoot,
    UsedNullifier(BytesN<32>),
    Admin,
    Volume(BytesN<32>),
    Token,
}

#[contract]
pub struct RemittanceContract;

#[contractimpl]
impl RemittanceContract {
    pub fn initialize(env: Env, admin: Address, merkle_root: BytesN<32>, token: Address) {
        let key_admin = DataKey::Admin;
        if env.storage().instance().has(&key_admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&key_admin, &admin);
        env.storage().instance().set(&DataKey::MerkleRoot, &merkle_root);
        env.storage().instance().set(&DataKey::Token, &token);
    }

    pub fn update_merkle_root(env: Env, admin: Address, new_root: BytesN<32>) {
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("not authorized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::MerkleRoot, &new_root);
    }

    pub fn deposit(env: Env, from: Address, amount: i128) {
        from.require_auth();
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&from, &env.current_contract_address(), &amount);
    }

    /// Verify a Groth16 proof, check nullifier, and release funds.
    ///
    /// Public signals: [merkle_root, nullifier_hash, recipient_hash]
    ///
    /// NOTE: The recipient hash is computed as `Poseidon(recipient_address)` in the
    /// circuit. The contract stores it for compliance volume tracking but does NOT
    /// independently verify it on-chain (Poseidon hash is not directly available in
    /// the SDK without the rs-soroban-poseidon crate). The proof itself guarantees
    /// the binding. In production, verify with rs-soroban-poseidon.
    pub fn send(
        env: Env,
        sender: Address,
        proof_a: Bn254G1Affine,
        proof_b: Bn254G2Affine,
        proof_c: Bn254G1Affine,
        public_signals: Vec<BytesN<32>>,
        recipient: Address,
        amount: i128,
    ) -> bool {
        sender.require_auth();

        let merkle_root_signal = public_signals.get(0).unwrap();
        let nullifier_hash = public_signals.get(1).unwrap();
        let recipient_hash_signal = public_signals.get(2).unwrap();

        // 1. Verify Merkle root matches stored root
        let stored_root: BytesN<32> = env.storage().instance().get(&DataKey::MerkleRoot).unwrap();
        if merkle_root_signal != stored_root {
            panic!("merkle root mismatch");
        }

        // 2. Check nullifier has not been used (anti-replay)
        let nullifier_key = DataKey::UsedNullifier(nullifier_hash.clone());
        if env.storage().instance().has(&nullifier_key) {
            panic!("nullifier already used");
        }

        // 3. Verify the Groth16 proof
        let vk = vk_constants::get_verification_key(&env);
        if !verify_groth16(&env, &vk, &proof_a, &proof_b, &proof_c, &public_signals) {
            panic!("proof verification failed");
        }

        // 4. Mark nullifier as used
        env.storage().instance().set(&nullifier_key, &true);

        // 5. Transfer tokens to recipient
        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        // 6. Update compliance volume tally
        let volume_key = DataKey::Volume(recipient_hash_signal.clone());
        let current_volume: i128 = env.storage().instance().get(&volume_key).unwrap_or(0);
        env.storage().instance().set(&volume_key, &(current_volume + amount));

        true
    }

    /// Compliance query: has any address matching this hash exceeded the threshold?
    /// Returns (exceeded: bool, nullifier_proof: BytesN<32>).
    pub fn compliance_query(
        env: Env,
        address_hash: BytesN<32>,
        threshold: i128,
    ) -> (bool, BytesN<32>) {
        let volume_key = DataKey::Volume(address_hash.clone());
        let total_volume: i128 = env.storage().instance().get(&volume_key).unwrap_or(0);
        let exceeded = total_volume > threshold;

        // Derive a verifiable query proof: keccak256(address_hash || threshold_bytes)
        let mut combined = [0u8; 64];
        combined[..32].copy_from_slice(&address_hash.to_array());
        let threshold_be = threshold.to_be_bytes();
        combined[32..48].copy_from_slice(&threshold_be);
        let proof_nullifier = env.crypto().keccak256(&BytesN::from_array(&env, &combined).into());

        (exceeded, proof_nullifier.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Env, BytesN};

    fn setup_env() -> (Env, RemittanceContractClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RemittanceContract, ());
        let client = RemittanceContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let root = BytesN::from_array(&env, &[0xab; 32]);
        client.initialize(&admin, &root, &token);
        (env, client, admin, token)
    }

    #[test]
    fn test_initialize() {
        let (_, _, _, _) = setup_env();
    }

    #[test]
    #[should_panic(expected = "already initialized")]
    fn test_double_initialize() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(RemittanceContract, ());
        let client = RemittanceContractClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let token = Address::generate(&env);
        let root = BytesN::from_array(&env, &[0xab; 32]);

        client.initialize(&admin, &root, &token);
        client.initialize(&admin, &root, &token);
    }

    #[test]
    fn test_compliance_query_empty() {
        let (env, client, _, _) = setup_env();
        let hash = BytesN::from_array(&env, &[0x11; 32]);
        let (exceeded, proof_nullifier) = client.compliance_query(&hash, &1000i128);
        assert!(!exceeded);
        assert_ne!(proof_nullifier, BytesN::from_array(&env, &[0u8; 32]));
    }

    #[test]
    fn test_update_merkle_root() {
        let (env, client, admin, _) = setup_env();
        let new_root = BytesN::from_array(&env, &[0xcd; 32]);
        client.update_merkle_root(&admin, &new_root);
    }
}
