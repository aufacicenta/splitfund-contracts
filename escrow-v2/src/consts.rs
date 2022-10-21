use near_sdk::{Balance, Gas};

// Fungible Token
pub const GAS_ON_TRANSFER: Gas = Gas(2_000_000_000_000);
pub const GAS_ON_TRANSFER_CB: Gas = Gas(2_000_000_000_000);
pub const BALANCE_ON_STORAGE_DEPOSIT: Balance = 1_250_000_000_000_000_000_000; // 0.00235 NEAR