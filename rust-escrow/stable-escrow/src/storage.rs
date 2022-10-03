use near_sdk::{
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::{LazyOption, UnorderedSet},
    near_bindgen,
    serde::{Deserialize, Serialize},
    AccountId,
};

use near_contract_standards::fungible_token::{metadata::FungibleTokenMetadata, FungibleToken};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    pub deposits: UnorderedSet<AccountId>, //@TODO Calculate storage for this
    pub ft: FungibleToken,
    pub ft_metadata: LazyOption<FungibleTokenMetadata>,
    pub metadata: Metadata,
}

#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub expires_at: u64,
    pub funding_amount_limit: u128,
    pub unpaid_amount: u128,
    pub nep_141_account_id: AccountId,
    pub dao_factory_account_id: AccountId,
    pub metadata_url: String,
}
