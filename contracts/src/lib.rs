#![no_std]

use soroban_sdk::crypto::bls12_381::{Bls12381Fr as Fr, Bls12381G1Affine as G1Affine, Bls12381G2Affine as G2Affine};
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

fn vk_alpha(env: &Env) -> G1Affine {
    G1Affine::from_array(env, &VK_ALPHA)
}
fn vk_beta(env: &Env) -> G2Affine {
    G2Affine::from_array(env, &VK_BETA)
}
fn vk_gamma(env: &Env) -> G2Affine {
    G2Affine::from_array(env, &VK_GAMMA)
}
fn vk_delta(env: &Env) -> G2Affine {
    G2Affine::from_array(env, &VK_DELTA)
}
fn vk_ic(env: &Env) -> Vec<G1Affine> {
    let mut v = Vec::new(env);
    for ic in VK_IC.iter() {
        v.push_back(G1Affine::from_array(env, ic));
    }
    v
}

// Paste these from circuits/build_bls/vk.hex — see instructions below
mod vk_constants;
use vk_constants::{VK_ALPHA, VK_BETA, VK_GAMMA, VK_DELTA, VK_IC};

fn verify_groth16(
    env: &Env,
    proof_a: &G1Affine,
    proof_b: &G2Affine,
    proof_c: &G1Affine,
    public_signals: &Vec<Fr>,
) -> bool {
    let bls = env.crypto().bls12_381();
    let ic = vk_ic(env);

    if public_signals.len() + 1 != ic.len() {
        return false;
    }

    let mut vk_x = ic.get(0).unwrap();
    for (s, v) in public_signals.iter().zip(ic.iter().skip(1)) {
        let prod = bls.g1_mul(&v, &s);
        vk_x = bls.g1_add(&vk_x, &prod);
    }

    let neg_a = -proof_a.clone();
    let vp1 = Vec::from_array(env, [neg_a, vk_alpha(env), vk_x, proof_c.clone()]);
    let vp2 = Vec::from_array(env, [proof_b.clone(), vk_beta(env), vk_gamma(env), vk_delta(env)]);

    bls.pairing_check(vp1, vp2)
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

    pub fn send(
        env: Env,
        sender: Address,
        proof_a: G1Affine,
        proof_b: G2Affine,
        proof_c: G1Affine,
        public_signals: Vec<Fr>,
        merkle_root_bytes: BytesN<32>,
        nullifier_hash_bytes: BytesN<32>,
        recipient_hash_bytes: BytesN<32>,
        recipient: Address,
        amount: i128,
    ) -> bool {
        sender.require_auth();

        let stored_root: BytesN<32> = env.storage().instance().get(&DataKey::MerkleRoot).unwrap();
        if merkle_root_bytes != stored_root {
            panic!("merkle root mismatch");
        }

        let nullifier_key = DataKey::UsedNullifier(nullifier_hash_bytes.clone());
        if env.storage().instance().has(&nullifier_key) {
            panic!("nullifier already used");
        }

        if !verify_groth16(&env, &proof_a, &proof_b, &proof_c, &public_signals) {
            panic!("proof verification failed");
        }

        env.storage().instance().set(&nullifier_key, &true);

        let token_addr: Address = env.storage().instance().get(&DataKey::Token).unwrap();
        let token_client = token::Client::new(&env, &token_addr);
        token_client.transfer(&env.current_contract_address(), &recipient, &amount);

        let volume_key = DataKey::Volume(recipient_hash_bytes.clone());
        let current_volume: i128 = env.storage().instance().get(&volume_key).unwrap_or(0);
        env.storage().instance().set(&volume_key, &(current_volume + amount));

        true
    }

    pub fn compliance_query(
        env: Env,
        address_hash: BytesN<32>,
        threshold: i128,
    ) -> (bool, BytesN<32>) {
        let volume_key = DataKey::Volume(address_hash.clone());
        let total_volume: i128 = env.storage().instance().get(&volume_key).unwrap_or(0);
        let exceeded = total_volume > threshold;

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
    use ark_bls12_381::{Fq, Fq2};
    use ark_serialize::CanonicalSerialize;
    use core::str::FromStr;
    use soroban_sdk::crypto::bls12_381::{G1Affine as BlsG1, G2Affine as BlsG2, Fr as BlsFr, G1_SERIALIZED_SIZE, G2_SERIALIZED_SIZE};
    use soroban_sdk::U256;

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