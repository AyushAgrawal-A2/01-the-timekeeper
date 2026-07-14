use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;

use crate::{Record, MAGIC, RECORD_SEED, TAG_RECORD};

pub struct WakeAccount<'a> {
    payer: &'a AccountView,
    record: &'a mut AccountView,
}
impl<'a> TryFrom<(&Address, &'a mut [AccountView])> for WakeAccount<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts): (&Address, &'a mut [AccountView]),
    ) -> Result<Self, Self::Error> {
        let [payer, record, _system, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (record_expected, record_bump) =
            Address::derive_program_address(&[RECORD_SEED, payer.address().as_ref()], program_id)
                .ok_or(ProgramError::InvalidSeeds)?;
        if *record.address() != record_expected {
            return Err(ProgramError::InvalidSeeds);
        }

        if !record.is_data_empty() {
            return Err(ProgramError::AccountAlreadyInitialized);
        }

        let record_bump = [record_bump];
        let seeds = [
            Seed::from(RECORD_SEED),
            Seed::from(payer.address().as_ref()),
            Seed::from(&record_bump),
        ];
        let signers = [Signer::from(&seeds)];
        create_account_with_minimum_balance_signed(
            record,
            Record::LEN,
            program_id,
            payer,
            None,
            &signers,
        )?;

        Ok(Self { payer, record })
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
        let accounts = WakeAccount::try_from((program_id, accounts))?;
        Ok(Self { accounts })
    }
}
impl<'a> Wake<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1u8;
    pub fn handle(&mut self) -> ProgramResult {
        let mut record_mut_ptr = self.accounts.record.try_borrow_mut()?;
        let record_data = Record::from_bytes_mut(record_mut_ptr.as_mut())?;
        record_data.set_inner(Record {
            magic: MAGIC,
            tag: TAG_RECORD,
            wallet: self.accounts.payer.address().to_bytes(),
            arrival_slot: Clock::get()?.slot.to_le_bytes(),
            attempts: 0u32.to_le_bytes(),
            solved: 0,
            solved_slot: [0u8; 8],
        });
        Ok(())
    }
}
