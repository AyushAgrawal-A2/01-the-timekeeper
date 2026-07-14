use std::{env, str::FromStr};

use solana_client::{rpc_client::RpcClient, rpc_config::CommitmentConfig};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::{keypair::read_keypair_file, Signer},
    transaction::Transaction,
};

use timekeeper::{compute_proof, Progress, CHIME_COUNT, GENESIS_SEED, ORACLE_SEED, PROGRESS_SEED};

const PROGRAM_ID: &str = "CEjNzYQz8ytqh2rG5azXeqBiA7TfPYWjJxYXh8ApaC9c";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = env::var("RPC").unwrap_or_else(|_| "https://api.devnet.solana.com".into());
    let rpc = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    let program_id = Pubkey::from_str(PROGRAM_ID)?;

    let keypair_path = env::args()
        .nth(1)
        .unwrap_or_else(|| format!("{}/.config/solana/id.json", env::var("HOME").unwrap()));
    let payer = read_keypair_file(&keypair_path).map_err(|e| format!("keypair {kp_path}: {e}"))?;
    println!("Wallet  : {}", payer.pubkey());
    println!("Balance : {} lamports", rpc.get_balance(&payer.pubkey())?);

    let (progress, _) =
        Pubkey::find_program_address(&[PROGRESS_SEED, payer.pubkey().as_ref()], &program_id);
    let (oracle, _) = Pubkey::find_program_address(&[ORACLE_SEED], &program_id);
    println!("Progress: {progress}");
    println!("Oracle  : {oracle}");

    let record = match rpc.get_account(&progress) {
        Ok(acc) => {
            println!("\n[wake] already awake — reusing existing record.");
            acc
        }
        Err(_) => {
            println!("\n[wake] recording your moment ...");
            let wake_ix = Instruction::new_with_bytes(
                program_id,
                &[1],
                vec![
                    AccountMeta::new(payer.pubkey(), true),
                    AccountMeta::new(progress, false),
                    AccountMeta::new_readonly(pinocchio_system::id(), false),
                ],
            );
            let tx = Transaction::new_signed_with_payer(
                &[wake_ix],
                Some(&payer.pubkey()),
                &[&payer],
                rpc.get_latest_blockhash()?,
            );
            let sig = rpc.send_and_confirm_transaction(&tx)?;
            println!("[wake] tx: {sig}");
            rpc.get_account(&progress)?
        }
    };

    let progress_data =
        Progress::from_bytes(&record.data).map_err(|_| "unexpected progress account data")?;
    if progress_data.is_solved() {
        println!("\nAlready cleared. The Timekeeper remembers you. Done.");
        return Ok(());
    };

    let proof = compute_proof(
        &payer.pubkey(),
        &GENESIS_SEED,
        CHIME_COUNT,
        &progress_data.arrival_slot,
    );
    let mut data = Vec::with_capacity(1 + proof.len());
    data.push(2);
    data.extend_from_slice(&proof);
    let clear_ix = Instruction::new_with_bytes(
        program_id,
        &data,
        vec![
            AccountMeta::new(payer.pubkey(), true),
            AccountMeta::new(progress, false),
            AccountMeta::new_readonly(oracle, false),
        ],
    );
    let tx = Transaction::new_signed_with_payer(
        &[clear_ix],
        Some(&payer.pubkey()),
        &[&payer],
        rpc.get_latest_blockhash()?,
    );

    let sim = rpc.simulate_transaction(&tx)?;
    println!("\n[clear] simulation err: {:?}", sim.value.err);
    if sim.value.err.is_some() {
        println!("logs: {:#?}", sim.value.logs);
        return Err("simulation failed — not sending".into());
    }
    let sig = rpc.send_and_confirm_transaction(&tx)?;
    println!("[clear] tx: {sig}");

    let after = rpc.get_account(&progress)?;
    let solved = Progress::from_bytes(&after.data)
        .map(|progress| progress.is_solved())
        .unwrap_or(false);
    println!(
        "\nsolved flag: {} {}",
        after.data[51],
        if solved {
            "✓ CLEARED — remembered on chain, for good."
        } else {
            "(unexpected)"
        }
    );
    Ok(())
}
