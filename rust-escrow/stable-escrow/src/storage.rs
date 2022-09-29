use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedMap},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId, Balance,
};

use near_contract_standards::fungible_token::{
    FungibleToken,
    metadata::FungibleTokenMetadata,
};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    pub deposits: UnorderedMap<AccountId, Balance>,
    pub ft: FungibleToken,
    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
    pub metadata: Metadata,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub expires_at: u64,
    pub funding_amount_limit: u128,
    pub unpaid_funding_amount: u128,
    pub nep_141_account_id: AccountId,
    pub dao_factory_account_id: AccountId,
    pub ft_factory_account_id: AccountId,
    pub dao_name: Option<String>,
    pub metadata_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct DepositArgs {}

#[derive(Serialize, Deserialize)]
pub enum Payload {
    DepositArgs(DepositArgs),
}
