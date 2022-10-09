use near_sdk::{env, log, near_bindgen, AccountId, Balance, PromiseResult};

use crate::storage::*;

#[near_bindgen]
impl Escrow {
    #[private]
    pub fn on_withdraw_callback(&mut self, receiver_id: AccountId, amount: String) -> Balance {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                let amount: Balance = amount.parse::<Balance>().unwrap();

                self.ft.internal_withdraw(&receiver_id, amount);
                self.deposits.remove(&receiver_id);
                self.metadata.unpaid_amount = self
                    .metadata
                    .unpaid_amount
                    .checked_add(amount)
                    .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

                log!("Successful Withdrawal. Account: {}, Amount: {}", receiver_id, amount);

                amount
            }
            _ => env::panic_str("ERR_WITHDRAW_UNSUCCESSFUL"),
        }
    }

    #[private]
    pub fn on_create_dao_callback(&mut self) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.metadata.dao_created = true;
                true
            }
            _ => env::panic_str("ERR_CREATE_DAO_UNSUCCESSFUL"),
        }
    }

    #[private]
    pub fn on_create_stake_callback(&mut self) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.metadata.stake_created = true;
                true
            }
            _ => env::panic_str("ERR_CREATE_STAKE_UNSUCCESSFUL"),
        }
    }

    #[private]
    pub fn on_create_proposals_callback(&mut self) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.metadata.dao_setuped = true;
                true
            }
            _ => env::panic_str("ERR_DAO_SETUP_UNSUCCESSFUL"),
        }
    }
}
