use near_contract_standards::fungible_token::{metadata::FungibleTokenMetadata, FungibleToken};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedSet},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance, StorageUsage
};
use ts_rs::TS;

pub type Timestamp = u64;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, TS)]
#[ts(export)]
pub struct Escrow {
    #[ts(type = "string[]")]
    pub deposits: UnorderedSet<AccountId>,
    #[ts(type = "FungibleToken")]
    pub ft: FungibleToken,
    #[ts(type = "FungibleTokenMetadata")]
    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
    pub metadata: Metadata,
    pub fees: Fees,
    pub account_storage_usage: StorageUsage,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Metadata {
    #[ts(type = "string")]
    pub expires_at: Timestamp,
    #[ts(type = "string")]
    pub funding_amount_limit: u128,
    #[ts(type = "string")]
    pub unpaid_amount: u128,
    #[ts(type = "string")]
    pub nep_141: AccountId,
    #[ts(type = "string")]
    pub maintainer_account_id: AccountId,
    pub metadata_url: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Fees {
    pub percentage: f32,
    #[ts(type = "string")]
    pub balance: Balance,
}
