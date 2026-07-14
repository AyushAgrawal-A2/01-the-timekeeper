#[cfg(test)]
mod test {
    use mollusk_svm::{
        program::keyed_account_for_system_program,
        result::{Check, InstructionResult},
        Mollusk,
    };
    use pinocchio::Address;
    use solana_sdk::{
        account::Account,
        message::{AccountMeta, Instruction},
    };
    use std::vec;

    use crate::{
        compute_proof, Progress, CHIME_COUNT, GENESIS_SEED, ORACLE_SEED, PROBLEM, PROGRESS_SEED,
        PROGRESS_TAG,
    };

    enum Target {
        Local,
        Devnet,
    }
    impl Target {
        fn path(&self) -> &'static str {
            match self {
                Self::Local => "target/deploy/timekeeper",
                Self::Devnet => "artifacts/timekeeper_devnet",
            }
        }
    }

    fn oracle_address(program_id: &Address) -> Address {
        Address::derive_program_address(&[ORACLE_SEED], program_id)
            .unwrap()
            .0
    }

    fn progress_address(payer: &Address, program_id: &Address) -> Address {
        Address::derive_program_address(&[PROGRESS_SEED, payer.as_ref()], program_id)
            .unwrap()
            .0
    }

    fn program_owned_account(program_id: &Address, data: &[u8]) -> Account {
        let mut account = Account::new(100_000_000, data.len(), &program_id);
        account.data = data.to_vec();
        account
    }

    fn initialize(
        mollusk: &mut Mollusk,
        program_id: Address,
        payer: Address,
        oracle: Address,
    ) -> InstructionResult {
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[0],
            vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(oracle, false),
                AccountMeta::new_readonly(pinocchio_system::ID, false),
            ],
        );
        let accounts = vec![
            (payer, Account::new(100_000_000, 0, &pinocchio_system::ID)),
            (oracle, Account::default()),
            keyed_account_for_system_program(),
        ];
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()])
    }

    fn wake(
        mollusk: &mut Mollusk,
        program_id: Address,
        payer: Address,
        progress: Address,
    ) -> InstructionResult {
        let instruction = Instruction::new_with_bytes(
            program_id,
            &[1],
            vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(progress, false),
                AccountMeta::new_readonly(pinocchio_system::ID, false),
            ],
        );
        let accounts = vec![
            (payer, Account::new(100_000_000, 0, &pinocchio_system::ID)),
            (progress, Account::default()),
            keyed_account_for_system_program(),
        ];
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()])
    }

    fn clear(
        mollusk: &mut Mollusk,
        program_id: Address,
        payer: Address,
        progress: Address,
        oracle: Address,
        progress_data: &[u8],
        oracle_data: &[u8],
        proof: &[u8],
    ) -> InstructionResult {
        let mut instruction_data = vec![2];
        instruction_data.extend_from_slice(proof);
        let instruction = Instruction::new_with_bytes(
            program_id,
            &instruction_data,
            vec![
                AccountMeta::new(payer, true),
                AccountMeta::new(progress, false),
                AccountMeta::new(oracle, false),
            ],
        );

        let accounts = vec![
            (payer, Account::new(100_000_000, 0, &pinocchio_system::ID)),
            (progress, program_owned_account(&program_id, progress_data)),
            (oracle, program_owned_account(&program_id, oracle_data)),
        ];
        mollusk.process_and_validate_instruction(&instruction, &accounts, &[Check::success()])
    }

    #[test]
    fn test_initialize() {
        let program_id = Address::new_unique();
        let payer = Address::new_unique();
        let oracle = oracle_address(&program_id);

        let genesis_slot = 475_292_775u64;

        let expected_oracle_data = include_bytes!("../artifacts/oracle.bin");

        let mut mollusk_local = Mollusk::new(&program_id, Target::Local.path());
        mollusk_local.warp_to_slot(genesis_slot);
        let result = initialize(&mut mollusk_local, program_id, payer, oracle);
        assert_eq!(
            result.get_account(&oracle).expect("oracle local").data,
            expected_oracle_data
        );

        let mut mollusk_devnet = Mollusk::new(&program_id, Target::Devnet.path());
        mollusk_devnet.warp_to_slot(genesis_slot);
        let result = initialize(&mut mollusk_devnet, program_id, payer, oracle);
        assert_eq!(
            result.get_account(&oracle).expect("oracle local").data,
            expected_oracle_data
        );
    }

    #[test]
    fn test_wake() {
        let program_id = Address::new_unique();
        let payer = Address::new_unique();
        let progress = progress_address(&payer, &program_id);

        let arrival_slot = 475_292_776u64;

        let mut expected_progress_data = [0u8; Progress::LEN];
        Progress::from_bytes_mut(&mut expected_progress_data)
            .unwrap()
            .set_inner(Progress {
                problem: PROBLEM,
                tag: PROGRESS_TAG,
                wallet: payer.to_bytes(),
                arrival_slot: 475_292_776u64.to_le_bytes(),
                attempts: 0u32.to_le_bytes(),
                solved: 0,
                solved_slot: [0u8; 8],
            });

        let mut mollusk_local = Mollusk::new(&program_id, Target::Local.path());
        mollusk_local.warp_to_slot(arrival_slot);
        let result = wake(&mut mollusk_local, program_id, payer, progress);
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &expected_progress_data
        );

        let mut mollusk_devnet = Mollusk::new(&program_id, Target::Devnet.path());
        mollusk_devnet.warp_to_slot(arrival_slot);
        let result = wake(&mut mollusk_devnet, program_id, payer, progress);
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &expected_progress_data
        );
    }

    #[test]
    fn test_clear() {
        let program_id = Address::new_unique();
        let payer = Address::new_unique();
        let progress = progress_address(&payer, &program_id);
        let oracle = oracle_address(&program_id);

        let arrival_slot = 475_292_776u64;
        let solve_slot = 475_292_776u64;

        let mut progress_data = [0u8; Progress::LEN];
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .set_inner(Progress {
                problem: PROBLEM,
                tag: PROGRESS_TAG,
                wallet: payer.to_bytes(),
                arrival_slot: arrival_slot.to_le_bytes(),
                attempts: 0u32.to_le_bytes(),
                solved: 0,
                solved_slot: [0u8; 8],
            });
        let oracle_data = include_bytes!("../artifacts/oracle.bin");

        let mut mollusk_local = Mollusk::new(&program_id, Target::Local.path());
        mollusk_local.warp_to_slot(solve_slot);

        let proof = [1; 40];
        let result = clear(
            &mut mollusk_local,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .increment_attempts()
            .unwrap();
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &progress_data
        );

        let proof = [1; 100];
        let result = clear(
            &mut mollusk_local,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .increment_attempts()
            .unwrap();
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &progress_data
        );

        let proof = compute_proof(
            &payer,
            &GENESIS_SEED,
            CHIME_COUNT,
            &arrival_slot.to_le_bytes(),
        );
        let result = clear(
            &mut mollusk_local,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .mark_solved(solve_slot);
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .increment_attempts()
            .unwrap();
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &progress_data
        );

        let proof = [1; 100];
        let result = clear(
            &mut mollusk_local,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &progress_data
        );

        let proof = compute_proof(
            &payer,
            &GENESIS_SEED,
            CHIME_COUNT,
            &arrival_slot.to_le_bytes(),
        );
        let result = clear(
            &mut mollusk_local,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        assert_eq!(
            result.get_account(&progress).expect("progress local").data,
            &progress_data
        );

        let mut mollusk_devnet = Mollusk::new(&program_id, Target::Devnet.path());
        mollusk_devnet.warp_to_slot(solve_slot);

        let proof = [1; 40];
        let result = clear(
            &mut mollusk_devnet,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .increment_attempts()
            .unwrap();
        assert_eq!(
            result.get_account(&progress).expect("progress devnet").data,
            &progress_data
        );

        let proof = [1; 32];
        let result = clear(
            &mut mollusk_devnet,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .increment_attempts()
            .unwrap();
        assert_eq!(
            result.get_account(&progress).expect("progress devnet").data,
            &progress_data
        );

        let proof = compute_proof(
            &payer,
            &GENESIS_SEED,
            CHIME_COUNT,
            &arrival_slot.to_le_bytes(),
        );
        let result = clear(
            &mut mollusk_devnet,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .mark_solved(solve_slot);
        Progress::from_bytes_mut(&mut progress_data)
            .unwrap()
            .increment_attempts()
            .unwrap();
        assert_eq!(
            result.get_account(&progress).expect("progress devnet").data,
            &progress_data
        );

        let proof = [1; 100];
        let result = clear(
            &mut mollusk_devnet,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        assert_eq!(
            result.get_account(&progress).expect("progress devnet").data,
            &progress_data
        );

        let proof = compute_proof(
            &payer,
            &GENESIS_SEED,
            CHIME_COUNT,
            &arrival_slot.to_le_bytes(),
        );
        let result = clear(
            &mut mollusk_devnet,
            program_id,
            payer,
            progress,
            oracle,
            &progress_data,
            oracle_data,
            &proof,
        );
        assert_eq!(
            result.get_account(&progress).expect("progress devnet").data,
            &progress_data
        );
    }
}
