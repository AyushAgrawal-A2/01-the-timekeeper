use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;

use crate::{
    Oracle, CHIME_COUNT, COMMITMENT, GENESIS_SEED, MESSAGE, ORACLE_SEED, ORACLE_TAG, PROBLEM,
};

pub struct InitializeAccount<'a> {
    oracle: &'a mut AccountView,
}
impl<'a> TryFrom<(&Address, &'a mut [AccountView])> for InitializeAccount<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts): (&Address, &'a mut [AccountView]),
    ) -> Result<Self, Self::Error> {
        let [payer, oracle, _system, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (oracle_expected, oracle_bump) =
            Address::derive_program_address(&[ORACLE_SEED], program_id)
                .ok_or(ProgramError::InvalidSeeds)?;
        if *oracle.address() != oracle_expected {
            return Err(ProgramError::InvalidSeeds);
        }

        let oracle_bump = [oracle_bump];
        let seeds = [Seed::from(ORACLE_SEED), Seed::from(&oracle_bump)];
        let signers = [Signer::from(&seeds)];
        create_account_with_minimum_balance_signed(
            oracle,
            Oracle::LEN,
            program_id,
            payer,
            None,
            &signers,
        )?;

        Ok(Self { oracle })
    }
}

pub struct Initialize<'a> {
    accounts: InitializeAccount<'a>,
}
impl<'a> TryFrom<(&'a Address, &'a mut [AccountView], &'a [u8])> for Initialize<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts, _data): (&'a Address, &'a mut [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: InitializeAccount::try_from((program_id, accounts))?,
        })
    }
}
impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0u8;
    pub fn handle(&mut self) -> ProgramResult {
        let mut oracle_mut_ptr = self.accounts.oracle.try_borrow_mut()?;
        let oracle_data = Oracle::from_bytes_mut(oracle_mut_ptr.as_mut())?;
        oracle_data.set_inner(&Oracle {
            problem: PROBLEM,
            tag: ORACLE_TAG,
            chime_count: CHIME_COUNT,
            genesis_seed: GENESIS_SEED,
            commitment: COMMITMENT,
            genesis_slot: Clock::get()?.slot.to_le_bytes(),
            message: MESSAGE,
        });
        Ok(())
    }
}
