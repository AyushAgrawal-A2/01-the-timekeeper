use pinocchio::Address;
use solana_nostd_sha256::hashv;

fn carry_forward(seed: &[u8; 32], count: u8) -> [u8; 32] {
    let mut cur = *seed;
    for _ in 0..count {
        cur = hashv(&[&cur]);
    }
    cur
}

pub fn compute_proof(
    user_address: &Address,
    genesis_seed: &[u8; 32],
    chime_count: u8,
    arrival_slot: &[u8; 8],
) -> [u8; 32] {
    let genesis_chime_hash = carry_forward(genesis_seed, chime_count);
    hashv(&[user_address.as_ref(), &genesis_chime_hash, arrival_slot])
}

fn constant_time_equal(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

pub fn is_proof_valid(
    user_address: &Address,
    genesis_seed: &[u8; 32],
    chime_count: u8,
    arrival_slot: &[u8; 8],
    user_proof: &[u8],
) -> bool {
    let expected_proof = compute_proof(user_address, genesis_seed, chime_count, arrival_slot);
    constant_time_equal(&expected_proof, user_proof)
}
