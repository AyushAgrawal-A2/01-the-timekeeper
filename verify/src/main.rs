//! Differential test harness for The Timekeeper.
//!
//! Loads BOTH the program dumped from devnet and the locally-compiled
//! reconstruction, runs the full instruction set against each, and byte-compares
//! every account state change between the two.
//!
//!   Part A — cross-program differential (original vs reconstruction):
//!            Wake, Wake-again, Clear-wrong, Clear-right, Clear-idempotent.
//!            After each step the record (and config) is compared byte-for-byte.
//!   Part B — reconstruction Initialize lifecycle: run tag-0, diff the produced
//!            config against the real devnet oracle, and confirm it's solvable.
//!   Part C — probe the original's Initialize (tag-0) for completeness.
//!
//! Defaults (override with args):
//!   verify <original.so> <reconstruction.so> <oracle.bin>

use anchor_lang::{
    solana_program::{
        clock::Clock,
        instruction::{AccountMeta, Instruction},
    },
    system_program,
};
use litesvm::LiteSVM;
use sha2::{Digest, Sha256};
use solana_account::Account;
use solana_keypair::{Address, Keypair};
use solana_signer::Signer;
use solana_transaction::Transaction;

use std::path::PathBuf;

const SLOT_WAKE: u64 = 476_015_426;
const SLOT_CLEAR: u64 = 476_023_976;

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}
fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{:02x}", x)).collect()
}

/// One program running in its own LiteSVM, funded, with the wallet we drive.
struct Prog {
    svm: LiteSVM,
    id: Address,
    wallet: Keypair,
}

impl Prog {
    fn new(name: [u8; 32], so: &[u8], wallet_secret: [u8; 32]) -> Self {
        let id = Address::new_from_array(name);
        let mut svm = LiteSVM::new();
        svm.add_program(id, so).unwrap();
        let wallet = Keypair::new_from_array(wallet_secret);
        svm.airdrop(&wallet.pubkey(), 10_000_000_000).unwrap();
        Prog { svm, id, wallet }
    }

    fn config_pda(&self) -> Address {
        Address::find_program_address(&[b"oracle"], &self.id).0
    }
    fn record_pda(&self) -> Address {
        Address::find_program_address(&[b"progress", self.wallet.pubkey().as_ref()], &self.id).0
    }

    fn set_config(&mut self, bytes: &[u8]) {
        let pk = self.config_pda();
        self.svm
            .set_account(
                pk,
                Account {
                    lamports: 5_000_000,
                    data: bytes.to_vec(),
                    owner: self.id,
                    executable: false,
                    rent_epoch: 0,
                },
            )
            .unwrap();
    }

    fn set_slot(&mut self, slot: u64) {
        let mut c: Clock = self.svm.get_sysvar();
        c.slot = slot;
        self.svm.set_sysvar(&c);
    }

    fn send(&mut self, ix: Instruction) -> Result<(), String> {
        self.svm.expire_blockhash();
        let bh = self.svm.latest_blockhash();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.wallet.pubkey()),
            &[&self.wallet],
            bh,
        );
        self.svm
            .send_transaction(tx)
            .map(|_| ())
            .map_err(|e| format!("{:?}", e.err))
    }

    fn wake(&mut self) -> Result<(), String> {
        let (w, r) = (self.wallet.pubkey(), self.record_pda());
        self.send(Instruction::new_with_bytes(
            self.id,
            &[1],
            vec![
                AccountMeta::new(w, true),
                AccountMeta::new(r, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        ))
    }

    fn clear(&mut self, proof: &[u8; 32]) -> Result<(), String> {
        let (w, r, c) = (self.wallet.pubkey(), self.record_pda(), self.config_pda());
        let mut data = vec![2u8];
        data.extend_from_slice(proof);
        self.send(Instruction::new_with_bytes(
            self.id,
            &data,
            vec![
                AccountMeta::new(w, true),
                AccountMeta::new(r, false),
                AccountMeta::new_readonly(c, false),
            ],
        ))
    }

    fn record_bytes(&self) -> Vec<u8> {
        self.svm
            .get_account(&self.record_pda())
            .map(|a| a.data)
            .unwrap_or_default()
    }
    fn config_bytes(&self) -> Vec<u8> {
        self.svm
            .get_account(&self.config_pda())
            .map(|a| a.data)
            .unwrap_or_default()
    }
}

/// Byte-compare two blobs; print a labeled PASS/FAIL and any differing offsets.
fn diff(label: &str, a: &[u8], b: &[u8], fails: &mut usize) {
    if a == b {
        println!("  PASS  {:<28} identical ({} bytes)", label, a.len());
        return;
    }
    *fails += 1;
    let n = a.len().max(b.len());
    let offs: Vec<usize> = (0..n).filter(|&i| a.get(i) != b.get(i)).collect();
    println!(
        "  FAIL  {:<28} len {}/{}  differing offsets: {:?}",
        label,
        a.len(),
        b.len(),
        offs
    );
    println!("          original: {}", hex(a));
    println!("          recon   : {}", hex(b));
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let args: Vec<String> = std::env::args().collect();
    let orig_path = args.get(1).cloned().unwrap_or_else(|| {
        root.join("artifacts/timekeeper_devnet.so")
            .to_string_lossy()
            .into()
    });
    let recon_path = args.get(2).cloned().unwrap_or_else(|| {
        root.join("../program/target/deploy/timekeeper.so")
            .to_string_lossy()
            .into()
    });
    let oracle_path = args
        .get(3)
        .cloned()
        .unwrap_or_else(|| root.join("artifacts/oracle.bin").to_string_lossy().into());

    let orig_so = std::fs::read(&orig_path).expect("read original .so");
    let recon_so = std::fs::read(&recon_path)
        .expect("read reconstruction .so (build the program first: cd program && cargo build-sbf)");
    let oracle = std::fs::read(&oracle_path).expect("read oracle.bin");

    println!("original .so     : {} ({} bytes)", orig_path, orig_so.len());
    println!(
        "reconstruction .so: {} ({} bytes)",
        recon_path,
        recon_so.len()
    );
    println!("oracle           : {} bytes\n", oracle.len());

    // Derive the proof purely from the oracle + our wallet + arrival slot.
    let seed: [u8; 32] = oracle[8..40].try_into().unwrap();
    let count = oracle[7];
    let mut kept = seed;
    for _ in 0..count {
        kept = sha256(&[&kept]);
    }

    // One shared wallet, used by both programs.
    let wallet = Keypair::new();
    let secret: [u8; 32] = wallet.to_bytes()[..32].try_into().unwrap();
    let proof = sha256(&[wallet.pubkey().as_ref(), &kept, &SLOT_WAKE.to_le_bytes()]);
    let wrong = [0xABu8; 32];

    let mut fails = 0usize;

    // ---- run the identical lifecycle against both programs ----
    // wake (once) -> clear wrong -> clear wrong -> clear RIGHT -> clear again
    // -> clear wrong again -> wake again (must be rejected). Snapshot after each.
    let run = |name: [u8; 32], so: &[u8]| -> (Vec<(String, Vec<u8>)>, String) {
        let mut p = Prog::new(name, so, secret);
        p.set_config(&oracle);
        p.set_slot(SLOT_WAKE);
        let mut s = vec![];
        p.wake().expect("wake #1 must succeed");
        s.push(("1 after wake         (record)".into(), p.record_bytes()));
        p.clear(&wrong).expect("clear ok");
        s.push(("2 after clear-wrong  (record)".into(), p.record_bytes()));
        p.clear(&wrong).expect("clear ok");
        s.push(("3 after clear-wrong  (record)".into(), p.record_bytes()));
        p.set_slot(SLOT_CLEAR);
        p.clear(&proof).expect("clear ok");
        s.push(("4 after clear-RIGHT  (record)".into(), p.record_bytes()));
        p.clear(&proof).expect("clear ok");
        s.push(("5 after clear-again  (record)".into(), p.record_bytes()));
        p.clear(&wrong).expect("clear ok");
        s.push(("6 after wrong-again  (record)".into(), p.record_bytes()));
        let wake_again = p
            .wake()
            .err()
            .unwrap_or_else(|| "<unexpectedly succeeded>".into());
        s.push(("7 config untouched".into(), p.config_bytes()));
        (s, wake_again)
    };

    println!("======== PART A: cross-program differential (all instructions) ========");
    let (a, aerr) = run([7u8; 32], &orig_so);
    let (b, berr) = run([9u8; 32], &recon_so);
    for ((la, va), (_lb, vb)) in a.iter().zip(b.iter()) {
        diff(la, va, vb, &mut fails);
    }
    if aerr == berr {
        println!("  PASS  {:<28} both reject re-wake: {}", "wake-again", aerr);
    } else {
        println!(
            "  FAIL  wake-again differs: original={:?}  recon={:?}",
            aerr, berr
        );
        fails += 1;
    }

    // Decode the solved record (step 4) so the byte blobs are legible.
    let fr = &a[3].1;
    println!(
        "\n  decoded record @step4: magic={:?} tag={} wallet={} arrival={} attempts={} solved={} solved_slot={}",
        String::from_utf8_lossy(&fr[0..6]), fr[6],
        &Address::new_from_array(fr[7..39].try_into().unwrap()).to_string()[..8],
        u64::from_le_bytes(fr[39..47].try_into().unwrap()),
        u32::from_le_bytes(fr[47..51].try_into().unwrap()),
        fr[51],
        u64::from_le_bytes(fr[52..60].try_into().unwrap()),
    );

    println!("\n======== PART B: Initialize differential (tag 0) ========");
    // The original's Initialize ignores its data and stamps genesis_slot from the
    // clock. Birth both oracles at the SAME slot (the real oracle's) so the whole
    // 498-byte config — commitment included — should come out byte-identical.
    let genesis_slot = u64::from_le_bytes(oracle[72..80].try_into().unwrap());
    let init_data = vec![0u8]; // tag only; both programs ignore the rest

    let birth = |name: [u8; 32], so: &[u8]| -> Result<Vec<u8>, String> {
        let mut p = Prog::new(name, so, secret);
        p.set_slot(genesis_slot);
        p.send(Instruction::new_with_bytes(
            p.id,
            &init_data,
            vec![
                AccountMeta::new(p.wallet.pubkey(), true),
                AccountMeta::new(p.config_pda(), false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        ))
        .map(|_| p.config_bytes())
    };

    let cfg_orig = birth([11u8; 32], &orig_so);
    let cfg_recon = birth([12u8; 32], &recon_so);

    match (&cfg_orig, &cfg_recon) {
        (Ok(co), Ok(cr)) => {
            println!(
                "  both Initializes minted a {}-byte config @ slot {}",
                co.len(),
                genesis_slot
            );
            diff("orig.Init  vs real oracle", co, &oracle, &mut fails);
            diff("recon.Init vs real oracle", cr, &oracle, &mut fails);
            diff("orig.Init  vs recon.Init", co, cr, &mut fails);
        }
        _ => {
            println!(
                "  FAIL  original Init ok: {}, recon Init ok: {}",
                cfg_orig.is_ok(),
                cfg_recon.is_ok()
            );
            fails += 1;
        }
    }

    // The freshly-initialized reconstruction oracle must be solvable.
    let recon_solvable = {
        let mut p = Prog::new([14u8; 32], &recon_so, secret);
        p.set_slot(genesis_slot);
        let _ = p.send(Instruction::new_with_bytes(
            p.id,
            &init_data,
            vec![
                AccountMeta::new(p.wallet.pubkey(), true),
                AccountMeta::new(p.config_pda(), false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
        ));
        p.set_slot(SLOT_WAKE);
        p.wake().is_ok() && {
            let arrival = u64::from_le_bytes(p.record_bytes()[39..47].try_into().unwrap());
            let pf = sha256(&[p.wallet.pubkey().as_ref(), &kept, &arrival.to_le_bytes()]);
            p.clear(&pf).is_ok() && p.record_bytes()[51] == 1
        }
    };
    println!(
        "  {}  {:<28} solved={}",
        if recon_solvable { "PASS" } else { "FAIL" },
        "recon fresh oracle solvable",
        recon_solvable
    );
    if !recon_solvable {
        fails += 1;
    }

    println!("\n=====================================================================");
    if fails == 0 {
        println!("ALL CHECKS PASSED — original and reconstruction are byte-equivalent.");
    } else {
        println!("{} CHECK(S) FAILED.", fails);
        std::process::exit(1);
    }
}
