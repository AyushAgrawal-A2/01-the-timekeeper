use pinocchio::{
    cpi::{Seed, Signer},
    error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    AccountView, Address, ProgramResult,
};
use pinocchio_system::create_account_with_minimum_balance_signed;

use crate::{
    Config, CHIME_COUNT, COMMITMENT, CONFIG_SEED, GENESIS_SEED, MAGIC, MESSAGE, TAG_CONFIG,
};

pub struct InitializeAccount<'a> {
    config: &'a mut AccountView,
}
impl<'a> TryFrom<(&Address, &'a mut [AccountView])> for InitializeAccount<'a> {
    type Error = ProgramError;
    fn try_from(
        (program_id, accounts): (&Address, &'a mut [AccountView]),
    ) -> Result<Self, Self::Error> {
        let [payer, config, _system, ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !payer.is_signer() {
            return Err(ProgramError::MissingRequiredSignature);
        }

        let (config_expected, config_bump) =
            Address::derive_program_address(&[CONFIG_SEED], program_id)
                .ok_or(ProgramError::InvalidSeeds)?;
        if *config.address() != config_expected {
            return Err(ProgramError::InvalidSeeds);
        }

        let config_bump = [config_bump];
        let seeds = [Seed::from(CONFIG_SEED), Seed::from(&config_bump)];
        let signers = [Signer::from(&seeds)];
        create_account_with_minimum_balance_signed(
            config,
            Config::LEN,
            program_id,
            payer,
            None,
            &signers,
        )?;

        Ok(Self { config })
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
        let accounts = InitializeAccount::try_from((program_id, accounts))?;
        Ok(Self { accounts })
    }
}
impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0u8;
    pub fn handle(&mut self) -> ProgramResult {
        let mut config_mut_ptr = self.accounts.config.try_borrow_mut()?;
        let config_data = Config::from_bytes_mut(config_mut_ptr.as_mut())?;
        config_data.set_inner(Config {
            magic: MAGIC,
            tag: TAG_CONFIG,
            chime_count: CHIME_COUNT,
            genesis_seed: GENESIS_SEED,
            commitment: COMMITMENT,
            genesis_slot: Clock::get()?.slot.to_le_bytes(),
            message: MESSAGE,
        });
        Ok(())
    }
}
