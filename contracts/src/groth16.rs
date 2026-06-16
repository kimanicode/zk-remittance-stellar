use soroban_sdk::crypto::bn254::{
    Bn254Fr, Bn254G1Affine, Bn254G2Affine,
};
use soroban_sdk::{vec, BytesN, Env, Vec, U256};

pub struct VerificationKey {
    pub alpha1: Bn254G1Affine,
    pub beta2: Bn254G2Affine,
    pub gamma2: Bn254G2Affine,
    pub delta2: Bn254G2Affine,
    pub ic: Vec<Bn254G1Affine>,
}

/// Compute X = IC[0] + sum(public_signals[i] * IC[i+1]) using MSM.
fn compute_vk_x(
    env: &Env,
    ic: &Vec<Bn254G1Affine>,
    public_signals: &Vec<BytesN<32>>,
) -> Bn254G1Affine {
    let n = public_signals.len();

    let mut points: Vec<Bn254G1Affine> = vec![env];
    let mut scalars: Vec<Bn254Fr> = vec![env];

    // IC[0] with scalar = 1
    points.push_back(ic.get(0).unwrap());
    scalars.push_back(Bn254Fr::from_u256(U256::from_u32(env, 1)));

    // IC[i+1] with scalar = public_signals[i] (as Bn254Fr)
    for i in 0..n {
        points.push_back(ic.get(i + 1).unwrap());
        let signal_bytes = public_signals.get(i).unwrap();
        let signal_fr = Bn254Fr::from_bytes(signal_bytes);
        scalars.push_back(signal_fr);
    }

    env.crypto().bn254().g1_msm(points, scalars)
}

/// Verify a Groth16 proof using BN254 pairings.
///
/// Returns `true` if the proof is valid.
///
/// The pairing equation checked is:
///   e(proof_a, proof_b) == e(α, β) * e(X, γ) * e(C, δ)
///
/// which we verify as:
///   e(proof_a, proof_b) * e(-α, β) * e(-X, γ) * e(-C, δ) == 1
pub fn verify_groth16(
    env: &Env,
    vk: &VerificationKey,
    proof_a: &Bn254G1Affine,
    proof_b: &Bn254G2Affine,
    proof_c: &Bn254G1Affine,
    public_signals: &Vec<BytesN<32>>,
) -> bool {
    // X = IC[0] + sum(s_i * IC[i+1])
    let vk_x = compute_vk_x(env, &vk.ic, public_signals);

    // Negate the G1 points on the right side of the pairing equation.
    let neg_alpha = -vk.alpha1.clone();
    let neg_vk_x = -vk_x;
    let neg_c = -proof_c.clone();

    // Build G1 and G2 vectors for the multi-pairing check.
    let mut g1_points: Vec<Bn254G1Affine> = vec![env];
    let mut g2_points: Vec<Bn254G2Affine> = vec![env];

    // e(A, B)
    g1_points.push_back(proof_a.clone());
    g2_points.push_back(proof_b.clone());

    // e(-α, β)
    g1_points.push_back(neg_alpha);
    g2_points.push_back(vk.beta2.clone());

    // e(-X, γ)
    g1_points.push_back(neg_vk_x);
    g2_points.push_back(vk.gamma2.clone());

    // e(-C, δ)
    g1_points.push_back(neg_c);
    g2_points.push_back(vk.delta2.clone());

    // Product of all pairings must equal 1 (identity in GT)
    env.crypto().bn254().pairing_check(g1_points, g2_points)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negate_g1_roundtrip() {
        let env = Env::default();

        let mut bytes = [0u8; 64];
        bytes[63] = 0x01;
        let point = Bn254G1Affine::from_bytes(BytesN::from_array(&env, &bytes));

        let neg = -point.clone();
        let _neg2 = -neg;
    }
}
