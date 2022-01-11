use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LookupMap;
use near_sdk::{env, log, near_bindgen};
use near_sdk::{AccountId, Balance, Promise};

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Escrow {
    deposits: LookupMap<AccountId, Balance>,
}

impl Default for Escrow {
    fn default() -> Self {
        env::panic_str("Escrow should be initialized before usage")
    }
}

#[near_bindgen]
impl Escrow {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            deposits: LookupMap::new(b"r".to_vec()),
        }
    }

    pub fn deposits_of(&self, payee: AccountId) -> Balance {
        return match self.deposits.get(&payee) {
            Some(deposit) => deposit,
            None => 0,
        };
    }

    #[payable]
    pub fn deposit(&mut self) {
        assert_ne!(
            env::predecessor_account_id(),
            env::signer_account_id(),
            "The owner of the contract should not deposit"
        );

        let amount = env::attached_deposit();
        let payee = env::signer_account_id();
        let current_balance = self.deposits_of(payee.clone());
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
        let payment = self.deposits_of(payee.clone());

        Promise::new(payee.clone()).transfer(payment);
        self.deposits.insert(&payee, &0);

        log!(
            "{} withdrawn {} NEAR tokens. New balance {}",
            payee,
            payment,
            self.deposits_of(payee.clone())
        );
        // @TODO emit withdraw event
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::{alice, bob, carol};
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::testing_env;

    const ATTACHED_DEPOSIT: Balance = 8540000000000000000000;

    fn setup_contract() -> (VMContextBuilder, Escrow) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.predecessor_account_id(alice()).build());
        let contract = Escrow::new();
        (context, contract)
    }

    #[test]
    fn test_get_deposits_of() {
        let (_context, contract) = setup_contract();
        assert_eq!(
            0,
            contract.deposits_of(alice()),
            "Account deposits should be 0"
        );
    }

    #[test]
    fn test_deposit() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            ATTACHED_DEPOSIT,
            contract.deposits_of(bob()),
            "Account deposits should equal ATTACHED_DEPOSIT"
        );
    }

    #[test]
    #[should_panic(expected = "The owner of the contract should not deposit")]
    fn test_owner_deposit() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .signer_account_id(alice())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();
    }

    #[test]
    fn test_withdraw() {
        let (mut context, mut contract) = setup_contract();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            ATTACHED_DEPOSIT,
            contract.deposits_of(carol()),
            "Account deposits should equal ATTACHED_DEPOSIT"
        );

        contract.withdraw();

        assert_eq!(
            0,
            contract.deposits_of(carol()),
            "Account deposits should equal 0"
        );
    }
}
