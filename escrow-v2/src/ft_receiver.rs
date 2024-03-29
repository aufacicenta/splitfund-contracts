use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::{env, json_types::U128, near_bindgen, AccountId, PromiseOrValue};

use crate::*;

#[near_bindgen]
impl FungibleTokenReceiver for Escrow {
    // `msg` doesn't affect the implementation for ft_on_transfer.
    #[allow(unused_variables)]
    #[payable]
    fn ft_on_transfer(
        &mut self,
        sender_id: AccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        if env::predecessor_account_id() != self.get_metadata().nep_141 {
            env::panic_str("ERR_WRONG_NEP141");
        }

        assert!(amount.0 > 0, "ERR_ZERO_AMOUNT");

        self.deposit(sender_id, amount.0);

        PromiseOrValue::Value(U128(0))
    }
}
