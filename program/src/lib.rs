#![no_std]

pub mod constant;
pub mod helpers;
pub mod instructions;
pub mod state;

pub use constant::*;
pub use helpers::*;
pub use instructions::*;
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
            Clear::try_from((program_id, accounts, data))?.handle()
        }
        Some((Wake::DISCRIMINATOR, data)) => {
            Clear::try_from((program_id, accounts, data))?.handle()
        }
        Some((Clear::DISCRIMINATOR, data)) => {
            Clear::try_from((program_id, accounts, data))?.handle()
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}
