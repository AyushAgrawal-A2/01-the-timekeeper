# The Timekeeper — reconstructed program

A faithful re-implementation of the on-chain challenge program
`CEjNzYQz8ytqh2rG5azXeqBiA7TfPYWjJxYXh8ApaC9c` (Solana devnet), rebuilt from the
deployed SBF bytecode and on-chain account state. It is behavior-compatible: same
PDA seeds, same account byte layouts, same proof, same silent-scoring — verified
against wallets that really solved it (`cargo test`).

## Instructions (first data byte = tag)

| Tag | Name | Accounts | Data |
|-----|------|----------|------|
| `0` | Initialize | `payer(s,w)`, `config_pda(w)`, `system` | ignored (constants are hardcoded; `genesis_slot` = current clock slot) |
| `1` | Wake | `signer(s,w)`, `record_pda(w)`, `system`, `clock` | — |
| `2` | Clear | `signer(s,w)`, `record_pda(w)`, `config_pda`, `system`, `clock` | `proof[32]` |

## PDAs

- **Config / oracle:** `find_program_address(["oracle"])`
- **Record:** `find_program_address(["progress", wallet])`

## Account layouts

**Config (498 bytes)** — `magic "TMKPR1"` · `tag=1` · `chime_count:u8` ·
`genesis_seed:[u8;32]` · `commitment:[u8;32]` · `genesis_slot:u64` · `message[418]`

**Record (60 bytes)** — `magic "TMKPR1"` · `tag=2` · `wallet:[u8;32]` ·
`arrival_slot:u64` · `attempts:u32` · `solved:u8` · `solved_slot:u64`

`attempts` is the number of `Clear` calls made while unsolved (your "marks").
`Wake` is create-only (a second `Wake` fails with `AccountAlreadyInitialized`);
`Clear` increments `attempts` on each unsolved try and is a no-op once solved.

## The proof — "the time it really keeps"

```
kept  = sha256^chime_count(genesis_seed)          // genesis carried forward (64 chimes)
proof = sha256( wallet ‖ kept ‖ arrival_slot_le ) // who you are ‖ kept time ‖ your first moment
```

The `genesis_slot` on the config's face is bait — the proof never uses it. A wrong
proof is **not** an error: `Clear` returns success without leaving a mark ("I score
in silence").

## Build / test / deploy

```bash
cargo test                     # fidelity tests vs. real on-chain data
cargo build-sbf                # -> target/deploy/timekeeper.so
solana program deploy target/deploy/timekeeper.so -u devnet
# then send tag-0 Initialize once to mint the oracle, and players use tag 1 / 2.
```

> Note: `commitment` is stored as `sha256("timekeeper::commit" ‖ genesis_seed)` — a
> deterministic stand-in for the original's 32-byte commitment field, which is
> flavor (the verification path never reads it). Everything the proof depends on is
> reproduced exactly.
