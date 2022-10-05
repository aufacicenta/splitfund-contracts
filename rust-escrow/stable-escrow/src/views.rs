use near_sdk::{env, near_bindgen, AccountId, Balance};

use crate::storage::*;

#[near_bindgen]
impl Escrow {
    /* You can use ft_balance_of instead
    pub fn get_shares_of(&self, payee: &AccountId) -> Balance {
        self.ft.internal_unwrap_balance_of(payee)
    }
    */

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

    pub fn get_deposit_accounts(&self) -> Vec<AccountId> {
        self.deposits.to_vec()
    }
}
