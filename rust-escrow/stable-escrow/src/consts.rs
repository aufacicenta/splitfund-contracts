use near_sdk::Gas;

/// Amount of gas
pub const GAS_FOR_CREATE_FT: Gas = Gas(50_000_000_000_000);
pub const GAS_FOR_PROPOSAL: Gas = Gas(25_000_000_000_000);
pub const GAS_FOR_CALLBACK: Gas = Gas(2_000_000_000_000);

// FT
pub const GAS_FT_TRANSFER: Gas = Gas(2_000_000_000_000);
pub const GAS_FT_WITHDRAW_CALLBACK: Gas = Gas(2_000_000_000_000);

// DAO Creation
// pub const BALANCE_FOR_CREATE_DAO: Balance = 6_000_000_000_000_000_000_000_000; // 6 Near
pub const GAS_FOR_CREATE_DAO: Gas = Gas(150_000_000_000_000); //@TODO validate the correct gas amount
pub const GAS_FOR_CREATE_DAO_CB: Gas = Gas(5_000_000_000_000);
pub const PROPOSAL_PERIOD: u64 = 604800000000000;

// Staking
// pub const BALANCE_FOR_CREATE_STAKE: Balance = 3_000_000_000_000_000_000_000_000; // 3 Near
pub const GAS_FOR_CREATE_STAKE: Gas = Gas(40_000_000_000_000);
pub const GAS_FOR_CREATE_STAKE_CB: Gas = Gas(5_000_000_000_000);
