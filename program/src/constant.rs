pub const MAGIC: [u8; 6] = *b"TMKPR1";

pub const TAG_CONFIG: u8 = 1;
pub const TAG_RECORD: u8 = 2;

pub const CONFIG_SEED: &[u8] = b"oracle";
pub const RECORD_SEED: &[u8] = b"progress";

/// The 32-byte seed the kept-time is derived from: sha256^CHIME_COUNT(GENESIS_SEED).
/// A random seed, not an address or text.
/// hex: 7bbdf95cc0a12e065a8c9e021994096c47cf39762f524a2489b7b024097c5844
pub const GENESIS_SEED: [u8; 32] = [
    0x7b, 0xbd, 0xf9, 0x5c, 0xc0, 0xa1, 0x2e, 0x06, 0x5a, 0x8c, 0x9e, 0x02, 0x19, 0x94, 0x09, 0x6c,
    0x47, 0xcf, 0x39, 0x76, 0x2f, 0x52, 0x4a, 0x24, 0x89, 0xb7, 0xb0, 0x24, 0x09, 0x7c, 0x58, 0x44,
];

/// How many chimes the kept-time is carried forward.
pub const CHIME_COUNT: u8 = 64;

/// The "committed clock" — a fixed 32-byte value baked into the original.
/// Flavor: the Clear verification never reads it.
/// hex: e24c74e2720b26169227fae1740c6debe1d1e6ceaed95b0a0c28d902534ba76d
pub const COMMITMENT: [u8; 32] = [
    0xe2, 0x4c, 0x74, 0xe2, 0x72, 0x0b, 0x26, 0x16, 0x92, 0x27, 0xfa, 0xe1, 0x74, 0x0c, 0x6d, 0xeb,
    0xe1, 0xd1, 0xe6, 0xce, 0xae, 0xd9, 0x5b, 0x0a, 0x0c, 0x28, 0xd9, 0x02, 0x53, 0x4b, 0xa7, 0x6d,
];

pub const MESSAGE: [u8; 418] = *b"TIMEKEEPER  //  challenge c1

A clock can show a tick without keeping that time.

I keep a genesis and a chime count. The tick you see is only what I
show. The time I keep is carried forward from what I remember.

Wake me once. I will write your moment under your wallet. Return with
a proof that binds you to the committed clock and that first moment.

I score in silence. A wrong proof earns no reason, only a mark.
";
