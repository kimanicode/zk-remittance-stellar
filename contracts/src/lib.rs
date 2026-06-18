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

    fn g1_from_coords(env: &Env, x: &str, y: &str) -> BlsG1 {
        let ark_g1 = ark_bls12_381::G1Affine::new(Fq::from_str(x).unwrap(), Fq::from_str(y).unwrap());
        let mut buf = [0u8; G1_SERIALIZED_SIZE];
        ark_g1.serialize_uncompressed(&mut buf[..]).unwrap();
        BlsG1::from_array(env, &buf)
    }

    fn g2_from_coords(env: &Env, x1: &str, x2: &str, y1: &str, y2: &str) -> BlsG2 {
        let x = Fq2::new(Fq::from_str(x1).unwrap(), Fq::from_str(x2).unwrap());
        let y = Fq2::new(Fq::from_str(y1).unwrap(), Fq::from_str(y2).unwrap());
        let ark_g2 = ark_bls12_381::G2Affine::new(x, y);
        let mut buf = [0u8; G2_SERIALIZED_SIZE];
        ark_g2.serialize_uncompressed(&mut buf[..]).unwrap();
        BlsG2::from_array(env, &buf)
    }

    fn fr_from_decimal(env: &Env, decimal: &str) -> BlsFr {
    extern crate std;
    use std::vec::Vec as StdVec;

    let mut digits: StdVec<u8> = decimal.bytes().map(|b| b - b'0').collect();
    let mut be_bytes = [0u8; 32];
    let mut idx = 32;
    while digits.iter().any(|&d| d != 0) {
        idx -= 1;
        let mut remainder = 0u32;
        let mut new_digits: StdVec<u8> = StdVec::with_capacity(digits.len());
        for &d in digits.iter() {
            let acc = remainder * 10 + d as u32;
            new_digits.push((acc / 256) as u8);
            remainder = acc % 256;
        }
        while new_digits.len() > 1 && new_digits[0] == 0 {
            new_digits.remove(0);
        }
        be_bytes[idx] = remainder as u8;
        digits = new_digits;
    }
    let u256 = U256::from_be_bytes(env, &soroban_sdk::Bytes::from_array(env, &be_bytes));
    BlsFr::from_u256(u256)
}

    #[test]
    fn test_real_proof_verifies() {
        let env = Env::default();

        let proof_a = g1_from_coords(
            &env,
            "505387286724809896153419658486968207792271809469232742356671033209831789071395220767618582584574036784467059953322",
            "2314874925631832125074206236355621576446799618317839496336095146062562417785552263693684020851308774660574592655492",
        );
        let proof_b = g2_from_coords(
            &env,
            "564528682345875868040534713056940128133317818734248223522366535920028137000455412378500005665095377451086606267772",
            "1824447613159836180191589467225833848366237930123637762202486032582316206232080504447936231932162656237605483451277",
            "165441449875583144405526853810182936630994851701104021995167498084711137679750705848681543472376838718792394538703",
            "3056624173584165212660964078056793908496867132054742465259189305012941042639232496184471587462592113476841275531112",
        );
        let proof_c = g1_from_coords(
            &env,
            "3448879368066358049625749150800341060640821325034647296690178109389735422360933608508016126457577910794338540729843",
            "413184489202805617387150851107604549948922495442587887251759441033435408320691074637535733566354482484770524332777",
        );

        let public_signals = Vec::from_array(
            &env,
            [
                fr_from_decimal(&env, "35589585953691632558272894079331513500970933280696499180108172190558787687138"),
                fr_from_decimal(&env, "35593111616396134186576814526464192374299791797224184116117225386017056746936"),
                fr_from_decimal(&env, "7846114547599950979977548495961514500109843146722585183135239779897529274437"),
            ],
        );

        let result = verify_groth16(&env, &proof_a, &proof_b, &proof_c, &public_signals);
        assert!(result, "real proof should verify successfully");
    }
}