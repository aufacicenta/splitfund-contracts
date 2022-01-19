use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::{assert_self, env, ext_contract, near_bindgen, AccountId, Gas, Promise, PublicKey};

const ESCROW_CODE: &[u8] = include_bytes!("./escrow.wasm");

/// Gas spent on the call & account creation.
const CREATE_CALL_GAS: Gas = Gas(75_000_000_000_000);

/// Gas allocated on the callback.
const ON_CREATE_CALL_GAS: Gas = Gas(10_000_000_000_000);

#[ext_contract(ext_self)]
pub trait ExtSelf {
    fn on_create_basic_escrow(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct EscrowFactory {
    escrow_contracts: UnorderedSet<AccountId>,
}

impl Default for EscrowFactory {
    fn default() -> Self {
        env::panic_str("EscrowFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl EscrowFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            escrow_contracts: UnorderedSet::new(b"d".to_vec()),
        }
    }

    pub fn get_basic_escrow_contracts_list(&self) -> Vec<AccountId> {
        self.escrow_contracts.to_vec()
    }

    /// Get number of created Basic Escrow Contracts.
    pub fn get_number_basic_escrow_contracts(&self) -> u64 {
        self.escrow_contracts.len()
    }

    /// Get Basic Escrow Contracts in paginated view.
    pub fn get_basic_escrow_contracts(&self, from_index: u64, limit: u64) -> Vec<AccountId> {
        let elements = self.escrow_contracts.as_vector();

        (from_index..std::cmp::min(from_index + limit, elements.len()))
            .filter_map(|index| elements.get(index))
            .collect()
    }

    #[payable]
    pub fn create_basic_escrow(
        &mut self,
        name: AccountId,
        public_key: Option<PublicKey>,
        args: Base64VecU8,
    ) -> Promise {
        let account_id: AccountId = format!("{}.{}", name, env::current_account_id())
            .parse()
            .unwrap();

        let mut promise = Promise::new(account_id.clone())
            .create_account()
            .deploy_contract(ESCROW_CODE.to_vec())
            .transfer(env::attached_deposit());

        if let Some(key) = public_key {
            promise = promise.add_full_access_key(key.into())
        }

        promise
            .function_call(
                "new".to_string(),
                args.into(),
                0,
                env::prepaid_gas() - CREATE_CALL_GAS - ON_CREATE_CALL_GAS,
            )
            .then(ext_self::on_create_basic_escrow(
                account_id,
                U128(env::attached_deposit()),
                env::predecessor_account_id(),
                env::current_account_id(),
                0,
                ON_CREATE_CALL_GAS,
            ))
    }

    pub fn on_create_basic_escrow(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool {
        assert_self();

        if near_sdk::is_promise_success() {
            self.escrow_contracts.insert(&account_id);
            true
        } else {
            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::alice;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, PromiseResult};

    fn setup_contract() -> (VMContextBuilder, EscrowFactory) {
        let mut context = VMContextBuilder::new();
        testing_env!(context.current_account_id(alice()).build());
        let factory = EscrowFactory::new();
        (context, factory)
    }

    #[test]
    fn create_basic_escrow() {
        let (mut context, mut factory) = setup_contract();

        factory.create_basic_escrow(
            "test".parse().unwrap(),
            Some(
                "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
                    .parse()
                    .unwrap(),
            ),
            "{}".as_bytes().to_vec().into(),
        );

        testing_env!(
            context.predecessor_account_id(alice()).build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        factory.on_create_basic_escrow(
            format!("test.{}", alice()).parse().unwrap(),
            U128(0),
            alice(),
        );

        assert_eq!(
            factory.get_basic_escrow_contracts_list(),
            vec![format!("test.{}", alice()).parse().unwrap()]
        );

        assert_eq!(
            factory.get_basic_escrow_contracts(0, 100),
            vec![format!("test.{}", alice()).parse().unwrap()]
        );

        assert_eq!(factory.get_number_basic_escrow_contracts(), 1);
    }
}
