use solana_nostd_sha256::hashv;

pub fn carry_forward(seed: &[u8; 32], count: u8) -> [u8; 32] {
    let mut cur = *seed;
    for _ in 0..count {
        cur = hashv(&[&cur]);
    }
    cur
}

pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..a.len() {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}
