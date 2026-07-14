use pinocchio::{error::ProgramError, ProgramResult};

use crate::MESSAGE;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Oracle {
    pub problem: [u8; 6],
    pub tag: u8,
    pub chime_count: u8,
    pub genesis_seed: [u8; 32],
    pub commitment: [u8; 32],
    pub genesis_slot: [u8; 8],
    pub message: [u8; MESSAGE.len()],
}
impl Oracle {
    pub const LEN: usize = core::mem::size_of::<Oracle>();
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }
    pub fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }
    pub fn set_inner(&mut self, other: Self) {
        self.problem = other.problem;
        self.tag = other.tag;
        self.chime_count = other.chime_count;
        self.genesis_seed = other.genesis_seed;
        self.commitment = other.commitment;
        self.genesis_slot = other.genesis_slot;
        self.message = other.message;
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Progress {
    pub problem: [u8; 6],
    pub tag: u8,
    pub wallet: [u8; 32],
    pub arrival_slot: [u8; 8],
    pub attempts: [u8; 4],
    pub solved: u8,
    pub solved_slot: [u8; 8],
}
impl Progress {
    pub const LEN: usize = core::mem::size_of::<Progress>();
    pub fn from_bytes(data: &[u8]) -> Result<&Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &*(data.as_ptr() as *const Self) })
    }
    pub fn from_bytes_mut(data: &mut [u8]) -> Result<&mut Self, ProgramError> {
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }
    pub fn set_inner(&mut self, other: Self) {
        self.problem = other.problem;
        self.tag = other.tag;
        self.wallet = other.wallet;
        self.arrival_slot = other.arrival_slot;
        self.attempts = other.attempts;
        self.solved = other.solved;
        self.solved_slot = other.solved_slot;
    }
    fn get_attempts(&self) -> u32 {
        u32::from_le_bytes(self.attempts)
    }
    pub fn increment_attempts(&mut self) -> ProgramResult {
        self.attempts = self
            .get_attempts()
            .checked_add(1)
            .ok_or(ProgramError::ArithmeticOverflow)?
            .to_le_bytes();
        Ok(())
    }
    pub fn is_solved(&self) -> bool {
        self.solved == 1
    }
    pub fn mark_solved(&mut self, slot: u64) {
        self.solved = 1;
        self.solved_slot = slot.to_le_bytes();
    }
}
