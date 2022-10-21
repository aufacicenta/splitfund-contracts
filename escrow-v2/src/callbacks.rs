use near_sdk::{env, json_types::U128, log, near_bindgen, AccountId, Balance, PromiseResult};

use crate::storage::*;

#[near_bindgen]
impl Escrow {
    #[private]
    pub fn on_withdraw_callback(&mut self, receiver_id: AccountId, amount: U128) -> Balance {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.ft.internal_withdraw(&receiver_id, amount.0);
                self.deposits.remove(&receiver_id);
                self.get_metadata().unpaid_amount = self
                    .metadata
                    .unpaid_amount
                    .checked_add(amount.0)
                    .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

                log!(
                    "[on_withdraw_callback]: receiver_id: {}, amount: {}",
                    receiver_id,
                    amount.0
                );

                amount.0
            }
            _ => env::panic_str("ERR_WITHDRAW_UNSUCCESSFUL"),
        }
    }

    #[private]
    pub fn on_claim_fees_callback(&mut self, amount: U128) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                log!(
                    "[on_claim_fees_callback]: fees_account_id: {}, claim: {}",
                    self.get_fees().account_id,
                    amount.0,
                );

                self.fees.claimed = true;
                true
            }
            _ => env::panic_str("ERR_CLAIM_FEES_UNSUCCESSFUL"),
        }
    }
}
