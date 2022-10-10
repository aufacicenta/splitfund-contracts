use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedSet},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance,
};

use near_contract_standards::fungible_token::{metadata::FungibleTokenMetadata, FungibleToken};

pub type Timestamp = u64;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    pub deposits: UnorderedSet<AccountId>, //@TODO Calculate storage for this
    pub ft: FungibleToken,
    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
    pub metadata: Metadata,
    pub dao: DAO,
    pub fees: Fees,
    pub staking: Staking,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub id: String,
    pub expires_at: u64,
    pub funding_amount_limit: u128,
    pub unpaid_amount: u128,
    pub nep_141: AccountId,
    pub maintainer_account_id: AccountId,
    pub metadata_url: String,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct DAO {
    pub created_at: Option<Timestamp>,
    pub setup_at: Option<Timestamp>,
    pub factory_account_id: AccountId,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Fees {
    pub percentage: f32,
    pub balance: Balance,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Staking {
    pub factory_account_id: AccountId,
    pub created_at: Option<Timestamp>,
}
