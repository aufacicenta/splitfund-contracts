use near_sdk::{Balance, Gas};

// FT
pub const GAS_FT_TRANSFER: Gas = Gas(2_000_000_000_000);
pub const GAS_FT_TRANSFER_CB: Gas = Gas(2_000_000_000_000);

// DAO Creation
pub const GAS_FOR_CREATE_DAO: Gas = Gas(150_000_000_000_000); //@TODO validate the correct gas amount
pub const GAS_FOR_CREATE_DAO_CB: Gas = Gas(5_000_000_000_000);
pub const PROPOSAL_PERIOD: u64 = 604800000000000;

// Staking
pub const GAS_FOR_CREATE_STAKE: Gas = Gas(40_000_000_000_000);
pub const GAS_FOR_CREATE_STAKE_CB: Gas = Gas(5_000_000_000_000);

// DAO Setup
pub const GAS_CREATE_DAO_PROPOSAL: Gas = Gas(10_000_000_000_000);
pub const GAS_CREATE_DAO_PROPOSAL_CB: Gas = Gas(2_000_000_000_000);
pub const BALANCE_PROPOSAL_BOND: Balance = 100_000_000_000_000_000_000_000; // 0.1 Near
