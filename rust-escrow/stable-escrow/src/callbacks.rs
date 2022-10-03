use near_sdk::{env, near_bindgen, AccountId, Balance, PromiseResult};

use crate::storage::*;

#[near_bindgen]
impl Escrow {
    #[private]
    pub fn on_withdraw_callback(&mut self, payee: AccountId, balance: String) -> Balance {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                let balance: Balance = balance.parse::<Balance>().unwrap();

                self.ft.internal_withdraw(&payee, balance);
                self.deposits.remove(&payee);
                self.metadata.unpaid_amount = self
                    .metadata
                    .unpaid_amount
                    .checked_add(balance)
                    .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

                // @TODO log

                balance
            }
            _ => env::panic_str("ERR_WITHDRAW_UNSUCCESSFUL"),
        }
    }
}
