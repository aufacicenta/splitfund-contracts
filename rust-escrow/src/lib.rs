use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, near_bindgen};
use near_sdk::{AccountId, Balance, Promise};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    deposits: LookupMap<AccountId, Balance>,
}

impl Default for Escrow {
    fn default() -> Self {
        Self {
            deposits: LookupMap::new(b"r".to_vec()),
        }
    }
}

#[near_bindgen]
impl Escrow {
    pub fn deposits_of(&self, payee: AccountId) -> Balance {
        return match self.deposits.get(&payee) {
            Some(deposit) => deposit,
            None => 0,
        };
    }

    #[payable]
    pub fn deposit(&mut self, payee: &AccountId) {
        let amount = env::attached_deposit();
        let current_balance = self.deposits_of(payee.to_string());
        self.deposits.insert(&payee, &(&current_balance + &amount));
        // @TODO emit deposit event
    }

    #[payable]
    pub fn withdraw(&mut self, payee: &AccountId) {
        let payment = self.deposits_of(payee.to_string());
        self.deposits.insert(&payee, &0);
        Promise::new(payee.to_string()).transfer(payment);
        // @TODO emit withdraw event
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn get_context(input: Vec<u8>, is_view: bool) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: "carol_near".to_string(),
            input,
            block_index: 0,
            block_timestamp: 0,
            account_balance: 0,
            account_locked_balance: 0,
            storage_usage: 0,
            attached_deposit: 0,
            prepaid_gas: 10u64.pow(18),
            random_seed: vec![0, 1, 2],
            is_view,
            output_data_receivers: vec![],
            epoch_height: 0,
        }
    }

    // Tests
    #[test]
    fn deposits_of() {
        let context = get_context(vec![], false);
        testing_env!(context);
        let contract = Escrow::default();
        assert_eq!(
            0,
            contract.deposits_of("bob_near".to_string()),
            "Account Balance should be 0"
        );
    }
}
