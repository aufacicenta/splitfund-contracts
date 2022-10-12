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
                self.get_metadata().unpaid_amount = self
                    .metadata
                    .unpaid_amount
                    .checked_add(amount)
                    .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

                log!(
                    "[on_withdraw_callback]: receiver_id: {}, amount: {}",
                    receiver_id,
                    amount
                );

                amount
            }
            _ => env::panic_str("ERR_WITHDRAW_UNSUCCESSFUL"),
        }
    }

    #[private]
    pub fn on_claim_fees_callback(&mut self, amount: String) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                let amount: Balance = amount.parse::<Balance>().unwrap();

                if amount > 0 {
                    self.ft.internal_deposit(
                        &self.get_metadata().maintainer_account_id.clone(),
                        amount,
                    );
                }

                log!(
                    "[on_claim_fees_callback]: maintainer: {}, claim: {}, DAO Investment {}",
                    self.get_metadata().maintainer_account_id,
                    self.get_fees().balance - amount,
                    amount,
                );

                //@TODO Burn unassigned tokens (fees)

                self.fees.balance = 0;
                true
            }
            _ => env::panic_str("ERR_CLAIM_FEES_UNSUCCESSFUL"),
        }
    }
}
