use pinocchio::{
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use solana_nostd_sha256::hashv;

use crate::{
    carry_forward, ct_eq, Config, Record, CONFIG_SEED, MAGIC, RECORD_SEED, TAG_CONFIG, TAG_RECORD,
};

pub struct ClearAccount<'a> {
    payer: &'a AccountView,
    record: &'a mut AccountView,
    config: &'a AccountView,
}
impl<'a> TryFrom<(&Address, &'a mut [AccountView])> for ClearAccount<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts): (&Address, &'a mut [AccountView]),
    ) -> Result<Self, Self::Error> {
        let [payer, record, config, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (record_expected, _) =
            Address::derive_program_address(&[RECORD_SEED, payer.address().as_ref()], program_id)
                .ok_or(ProgramError::InvalidSeeds)?;
        let (config_expected, _) = Address::derive_program_address(&[CONFIG_SEED], program_id)
            .ok_or(ProgramError::InvalidSeeds)?;
        if *record.address() != record_expected || *config.address() != config_expected {
            return Err(ProgramError::InvalidSeeds);
        }
        if !record.owned_by(program_id) || !config.owned_by(program_id) {
            return Err(ProgramError::IllegalOwner);
        }

        {
            let config_ptr = config.try_borrow()?;
            let config_data = Config::from_bytes(config_ptr.as_ref())?;
            if config_data.magic != MAGIC || config_data.tag != TAG_CONFIG {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        {
            let record_ptr = record.try_borrow()?;
            let record_data = Record::from_bytes(record_ptr.as_ref())?;
            if record_data.magic != MAGIC
                || record_data.tag != TAG_RECORD
                || record_data.wallet != payer.address().to_bytes()
            {
                return Err(ProgramError::InvalidAccountData);
            }
        }

        Ok(Self {
            payer,
            record,
            config,
        })
    }
}

pub struct ClearInstructionData<'a> {
    data: &'a [u8],
}
impl<'a> TryFrom<&'a [u8]> for ClearInstructionData<'a> {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != 32 {
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
        let accounts = ClearAccount::try_from((program_id, accounts))?;
        let instruction_data = ClearInstructionData::try_from(data)?;
        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}
impl<'a> Clear<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2u8;
    pub fn handle(&mut self) -> ProgramResult {
        let mut record_mut_ptr = self.accounts.record.try_borrow_mut()?;
        let record_data = Record::from_bytes_mut(record_mut_ptr.as_mut())?;
        if record_data.is_solved() {
            return Ok(());
        }

        record_data.increment_attempts()?;

        // Recover "the time it really keeps": carry the genesis forward chime_count times.
        let kept = {
            let config_ptr = self.accounts.config.try_borrow()?;
            let config_data = Config::from_bytes(config_ptr.as_ref())?;
            if config_data.magic != MAGIC || config_data.tag != TAG_CONFIG {
                return Err(ProgramError::InvalidAccountData);
            }
            carry_forward(&config_data.genesis_seed, config_data.chime_count)
        };

        // The proof binds who you are, the kept time, and your first moment.
        let expected = hashv(&[
            self.accounts.payer.address().as_ref(),
            &kept,
            &record_data.arrival_slot,
        ]);
        if ct_eq(&self.instruction_data.data[0..32], &expected) {
            record_data.mark_solved(Clock::get()?.slot);
        }
        Ok(())
    }
}
