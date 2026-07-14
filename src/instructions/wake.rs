use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;

use crate::{Progress, PROBLEM, PROGRESS_SEED, PROGRESS_TAG};

pub struct WakeAccount<'a> {
    payer: &'a AccountView,
    progress: &'a mut AccountView,
}
impl<'a> TryFrom<(&Address, &'a mut [AccountView])> for WakeAccount<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts): (&Address, &'a mut [AccountView]),
    ) -> Result<Self, Self::Error> {
        let [payer, progress, _system, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (progress_expected, progress_bump) =
            Address::derive_program_address(&[PROGRESS_SEED, payer.address().as_ref()], program_id)
                .ok_or(ProgramError::InvalidSeeds)?;
        if *progress.address() != progress_expected {
            return Err(ProgramError::InvalidSeeds);
        }

        if !progress.is_data_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let progress_bump = [progress_bump];
        let seeds = [
            Seed::from(PROGRESS_SEED),
            Seed::from(payer.address().as_ref()),
            Seed::from(&progress_bump),
        ];
        let signers = [Signer::from(&seeds)];
        create_account_with_minimum_balance_signed(
            progress,
            Progress::LEN,
            program_id,
            payer,
            None,
            &signers,
        )?;

        Ok(Self { payer, progress })
    }
}

pub struct Wake<'a> {
    accounts: WakeAccount<'a>,
}
impl<'a> TryFrom<(&'a Address, &'a mut [AccountView], &'a [u8])> for Wake<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts, _data): (&'a Address, &'a mut [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: WakeAccount::try_from((program_id, accounts))?,
        })
    }
}
impl<'a> Wake<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1u8;
    pub fn handle(&mut self) -> ProgramResult {
        let mut progress_mut_ptr = self.accounts.progress.try_borrow_mut()?;
        let progress_data = Progress::from_bytes_mut(progress_mut_ptr.as_mut())?;
        progress_data.set_inner(&Progress {
            problem: PROBLEM,
            tag: PROGRESS_TAG,
            wallet: self.accounts.payer.address().to_bytes(),
            arrival_slot: Clock::get()?.slot.to_le_bytes(),
            attempts: 0u32.to_le_bytes(),
            solved: 0,
            solved_slot: [0u8; 8],
        });
        Ok(())
    }
}
