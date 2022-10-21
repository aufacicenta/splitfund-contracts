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
    pub fn on_claim_fees_callback(&mut self, fees_to_invest: U128) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                // If fees_to_invest > 0, the escrow is successful
                // Half of the fees are collected and half invested in the asset
                if fees_to_invest.0 > 0 {
                    self.ft.internal_deposit(
                        &self.get_metadata().maintainer_account_id.clone(),
                        fees_to_invest.0,
                    );
                }

                log!(
                    "[on_claim_fees_callback]: fees_account_id: {}, claim: {}, Investment {}",
                    self.get_fees().account_id,
                    self.get_fees().amount - fees_to_invest.0,
                    fees_to_invest.0,
                );

                //@TODO Burn unassigned tokens (fees)

                self.fees.claimed = true;
                true
            }
            _ => env::panic_str("ERR_CLAIM_FEES_UNSUCCESSFUL"),
        }
    }
}
