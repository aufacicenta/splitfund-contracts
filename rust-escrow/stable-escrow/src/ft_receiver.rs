use near_sdk::{env, near_bindgen, serde_json, AccountId, Balance};

use crate::*;

pub trait FungibleTokenReceiver {
    // @returns amount of unused tokens
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: String, msg: String) -> String;
}

#[near_bindgen]
impl FungibleTokenReceiver for Escrow {
    /**
     * @notice a callback function only callable by the collateral token for this market
     * @param sender_id the sender of the original transaction
     * @param amount of tokens attached to this callback call
     * @param msg can be a string of any type, in this case we expect a stringified json object
     * @returns the amount of tokens that were not spent
     */
    #[payable]
    fn ft_on_transfer(&mut self, sender_id: AccountId, amount: String, msg: String) -> String {
        if !env::state_exists() {
            env::panic_str("ERR_NOT_INITIALIZED");
        }

        let amount: Balance = amount.parse::<Balance>().unwrap();
        assert!(amount > 0, "ERR_ZERO_AMOUNT");

        let payload: Payload = serde_json::from_str(&msg).expect("ERR_INVALID_PAYLOAD_MSG");

        match payload {
            Payload::DepositArgs(_payload) => self.deposit(sender_id, amount),
        };

        // All the collateral was used, so we should issue no refund on ft_resolve_transfer
        return "0".to_string();
    }
}
