#![no_std]

pub mod constant;
pub mod instructions;
pub mod proof;
pub mod state;
#[cfg(test)]
pub mod tests;
#[cfg(test)]
extern crate std;

pub use constant::*;
pub use instructions::*;
pub use proof::*;
pub use state::*;

use pinocchio::{
    entrypoint, error::ProgramError, nostd_panic_handler, AccountView, Address, ProgramResult,
};

entrypoint!(process_instruction);
nostd_panic_handler!();

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    match data.split_first() {
        Some((Initialize::DISCRIMINATOR, data)) => {
            Initialize::try_from((program_id, accounts, data))?.handle()
        }
        Some((Wake::DISCRIMINATOR, data)) => Wake::try_from((program_id, accounts, data))?.handle(),
        Some((Clear::DISCRIMINATOR, data)) => {
            Clear::try_from((program_id, accounts, data))?.handle()
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
