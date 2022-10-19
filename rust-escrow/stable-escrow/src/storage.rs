use near_contract_standards::fungible_token::{metadata::FungibleTokenMetadata, FungibleToken};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedSet},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance, StorageUsage,
};

pub type Timestamp = u64;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    pub deposits: UnorderedSet<AccountId>,
    pub ft: FungibleToken,
    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
    pub metadata: Metadata,
    pub fees: Fees,
    pub account_storage_usage: StorageUsage,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub expires_at: Timestamp,
    pub funding_amount_limit: u128,
    pub unpaid_amount: u128,
    pub nep_141: AccountId,
    pub maintainer_account_id: AccountId,
    pub metadata_url: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Fees {
    pub percentage: f32,
    pub balance: Balance,
}
