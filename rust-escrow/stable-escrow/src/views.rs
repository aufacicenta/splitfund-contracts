use near_sdk::{env, near_bindgen, AccountId, Balance};

use crate::storage::*;

#[near_bindgen]
impl Escrow {
    pub fn get_total_funds(&self) -> Balance {
        self.metadata.funding_amount_limit - self.metadata.unpaid_amount
    }

    pub fn get_metadata_url(&self) -> String {
        self.metadata.metadata_url.clone()
    }

    pub fn get_expiration_date(&self) -> u64 {
        self.metadata.expires_at
    }

    pub fn get_funding_amount_limit(&self) -> u128 {
        self.metadata.funding_amount_limit
    }

    pub fn get_unpaid_amount(&self) -> u128 {
        self.metadata.unpaid_amount
    }

    pub fn get_dao_factory_account_id(&self) -> AccountId {
        self.metadata.dao_factory.clone()
    }

    pub fn is_deposit_allowed(&self) -> bool {
        !self.has_contract_expired() && !self.is_funding_reached()
    }

    pub fn is_withdrawal_allowed(&self) -> bool {
        self.has_contract_expired() && !self.is_funding_reached()
    }

    pub fn has_contract_expired(&self) -> bool {
        self.metadata.expires_at < env::block_timestamp().try_into().unwrap()
    }

    pub fn is_funding_reached(&self) -> bool {
        self.get_total_funds() >= self.get_funding_amount_limit()
    }

    pub fn get_deposit_accounts(&self) -> Vec<String> {
        let mut accounts = vec![];

        for i in self.deposits.to_vec() {
            accounts.push(i.to_string());
        }

        accounts
    }

    pub fn is_dao_created(&self) -> bool {
        self.metadata.dao_created
    }

    pub fn is_dao_setuped(&self) -> bool {
        self.metadata.dao_setuped
    }

    pub fn is_stake_created(&self) -> bool {
        self.metadata.stake_created
    }

    pub fn get_fee_percentage(&self) -> f32 {
        self.metadata.fee_percentage
    }

    pub fn get_fee_balance(&self) -> Balance {
        self.metadata.fee_balance
    }
}
