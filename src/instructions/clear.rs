use pinocchio::{
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};

use crate::{
    is_proof_valid, Oracle, Progress, ORACLE_SEED, ORACLE_TAG, PROBLEM, PROGRESS_SEED, PROGRESS_TAG,
};

pub struct ClearAccount<'a> {
    payer: &'a AccountView,
    progress: &'a mut AccountView,
    oracle: &'a AccountView,
}
impl<'a> TryFrom<(&Address, &'a mut [AccountView])> for ClearAccount<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts): (&Address, &'a mut [AccountView]),
    ) -> Result<Self, Self::Error> {
        let [payer, progress, oracle, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (progress_expected, _) =
            Address::derive_program_address(&[PROGRESS_SEED, payer.address().as_ref()], program_id)
                .ok_or(ProgramError::InvalidSeeds)?;
        let (oracle_expected, _) = Address::derive_program_address(&[ORACLE_SEED], program_id)
            .ok_or(ProgramError::InvalidSeeds)?;
        if *progress.address() != progress_expected || *oracle.address() != oracle_expected {
            return Err(ProgramError::InvalidSeeds);
        }
        if !progress.owned_by(program_id) || !oracle.owned_by(program_id) {
            return Err(ProgramError::IllegalOwner);
        }

        {
            let progress_ptr = progress.try_borrow()?;
            let progress_data = Progress::from_bytes(progress_ptr.as_ref())?;
            if progress_data.problem != PROBLEM
                || progress_data.tag != PROGRESS_TAG
                || progress_data.wallet != payer.address().to_bytes()
            {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        {
            let oracle_ptr = oracle.try_borrow()?;
            let oracle_data = Oracle::from_bytes(oracle_ptr.as_ref())?;
            if oracle_data.problem != PROBLEM || oracle_data.tag != ORACLE_TAG {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        Ok(Self {
            payer,
            progress,
            oracle,
        })
    }
}

pub struct ClearInstructionData<'a> {
    data: &'a [u8],
}
impl<'a> TryFrom<&'a [u8]> for ClearInstructionData<'a> {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() < 32 {
            return Err(ProgramError::InvalidInstructionData);
        }
        Ok(Self { data })
    }
}

pub struct Clear<'a> {
    accounts: ClearAccount<'a>,
    instruction_data: ClearInstructionData<'a>,
}
impl<'a> TryFrom<(&'a Address, &'a mut [AccountView], &'a [u8])> for Clear<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts, data): (&'a Address, &'a mut [AccountView], &'a [u8]),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            accounts: ClearAccount::try_from((program_id, accounts))?,
            instruction_data: ClearInstructionData::try_from(data)?,
        })
    }
}
impl<'a> Clear<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2u8;
    pub fn handle(&mut self) -> ProgramResult {
        let mut progress_mut_ptr = self.accounts.progress.try_borrow_mut()?;
        let progress_data = Progress::from_bytes_mut(progress_mut_ptr.as_mut())?;
        if progress_data.is_solved() {
            return Ok(());
        }
        progress_data.increment_attempts()?;

        if self.instruction_data.data.len() >= 32 {
            let oracle_ptr = self.accounts.oracle.try_borrow()?;
            let oracle_data = Oracle::from_bytes(oracle_ptr.as_ref())?;
            if is_proof_valid(
                self.accounts.payer.address(),
                &oracle_data.genesis_seed,
                oracle_data.chime_count,
                &progress_data.arrival_slot,
                &self.instruction_data.data[0..32],
            ) {
                progress_data.mark_solved(Clock::get()?.slot);
            }
        }

        Ok(())
    }
}
