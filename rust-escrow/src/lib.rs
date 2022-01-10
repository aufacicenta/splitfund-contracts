use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, log, near_bindgen};
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
    pub fn deposit(&mut self) {
        let amount = env::attached_deposit();
        let payee = env::signer_account_id();
        let current_balance = self.deposits_of(payee.to_string());
        let new_balance = &(&current_balance + &amount);

        self.deposits.insert(&payee, new_balance);

        log!(
            "{} deposited {} NEAR tokens. New balance {}",
            payee,
            amount,
            new_balance
        );
        // @TODO emit deposit event
    }

    #[payable]
    pub fn withdraw(&mut self) {
        let payee = env::signer_account_id();
        let payment = self.deposits_of(payee.to_string());

        self.deposits.insert(&payee, &0);

        Promise::new(payee.to_string()).transfer(payment);

        log!(
            "{} withdrawn {} NEAR tokens. New balance {}",
            payee,
            payment,
            self.deposits_of(payee.to_string())
        );
        // @TODO emit withdraw event
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    use near_sdk::MockedBlockchain;

    const ATTACHED_DEPOSIT: Balance = 8540000000000000000000;

    fn setup_contract() -> (VMContextBuilder, Escrow) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(accounts(0)).build());
        let contract = Escrow::default();
        (context, contract)
    }

    #[test]
    fn test_get_deposits_of() {
        let (_context, contract) = setup_contract();
        assert_eq!(
            0,
            contract.deposits_of(accounts(0).to_string()),
            "Account deposits should be 0"
        );
    }

    #[test]
    fn test_deposit() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .signer_account_id(accounts(1))
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            ATTACHED_DEPOSIT,
            contract.deposits_of(accounts(1).to_string()),
            "Account deposits should equal ATTACHED_DEPOSIT"
        );
    }

    #[test]
    fn test_withdraw() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .signer_account_id(accounts(1))
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            ATTACHED_DEPOSIT,
            contract.deposits_of(accounts(1).to_string()),
            "Account deposits should equal ATTACHED_DEPOSIT"
        );

        contract.withdraw();

        assert_eq!(
            0,
            contract.deposits_of(accounts(1).to_string()),
            "Account deposits should equal 0"
        );
    }
}
