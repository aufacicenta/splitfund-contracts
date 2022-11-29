use near_sdk::{env, near_bindgen, Balance};

use crate::storage::*;

#[near_bindgen]
impl Splitfund {
    pub fn get_total_funds(&self) -> Balance {
        self.get_metadata().funding_amount_limit - self.get_metadata().unpaid_amount
    }

    pub fn is_deposit_allowed(&self) -> bool {
        !self.has_contract_expired() && !self.is_funding_reached()
    }

    pub fn is_withdrawal_allowed(&self) -> bool {
        self.has_contract_expired() && !self.is_funding_reached()
    }

    pub fn has_contract_expired(&self) -> bool {
        self.get_metadata().expires_at < self.get_block_timestamp()
    }

    pub fn is_funding_reached(&self) -> bool {
        self.get_total_funds() >= self.get_metadata().funding_amount_limit
    }

    pub fn get_deposit_accounts(&self) -> Vec<String> {
        let mut accounts = vec![];

        for i in self.deposits.to_vec() {
            accounts.push(i.to_string());
        }

        accounts
    }

    pub fn get_fees(&self) -> Fees {
        self.fees.clone()
    }

    pub fn get_metadata(&self) -> Metadata {
        self.metadata.clone()
    }

    pub fn get_block_timestamp(&self) -> u64 {
        env::block_timestamp()
    }
}
