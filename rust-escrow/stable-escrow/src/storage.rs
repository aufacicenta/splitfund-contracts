use near_contract_standards::fungible_token::{metadata::FungibleTokenMetadata, FungibleToken};
use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedSet},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance,
};
use ts_rs::TS;

pub type Timestamp = u64;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, TS)]
#[ts(export)]
pub struct Escrow {
    #[ts(type = "string[]")]
    pub deposits: UnorderedSet<AccountId>, //@TODO Calculate storage for this
    #[ts(type = "FungibleToken")]
    pub ft: FungibleToken,
    #[ts(type = "FungibleTokenMetadata")]
    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
    pub metadata: Metadata,
    pub dao: DAO,
    pub fees: Fees,
    pub staking: Staking,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Metadata {
    pub id: String,
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
pub struct DAO {
    #[ts(type = "string")]
    pub created_at: Option<Timestamp>,
    #[ts(type = "string")]
    pub setup_at: Option<Timestamp>,
    #[ts(type = "string")]
    pub factory_account_id: AccountId,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Fees {
    pub percentage: f32,
    #[ts(type = "string")]
    pub balance: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone, TS)]
#[ts(export)]
pub struct Staking {
    #[ts(type = "string")]
    pub factory_account_id: AccountId,
    #[ts(type = "string")]
    pub created_at: Option<Timestamp>,
}
