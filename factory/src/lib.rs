use near_sdk::{
    assert_self,
    borsh::{self, BorshDeserialize, BorshSerialize},
    collections::UnorderedSet,
    env,
    json_types::{Base64VecU8, U128},
    near_bindgen,
    serde_json::json,
    AccountId, Gas, Promise,
};

const ESCROW_CODE: &[u8] = include_bytes!("../../escrow-v2/res/escrow.wasm");

/// Gas spent on the call & account creation.
const CREATE_CALL_GAS: Gas = Gas(75_000_000_000_000);

/// Gas allocated on the callback.
const ON_CREATE_CALL_GAS: Gas = Gas(10_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct EscrowFactory {
    escrows: UnorderedSet<AccountId>,
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
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

        Self {
            escrows: UnorderedSet::new(b"d".to_vec()),
        }
    }

    #[payable]
    pub fn create_escrow(&mut self, name: String, args: Base64VecU8) -> Promise {
        let account_id: AccountId = format!("{}.{}", name, env::current_account_id())
            .parse()
            .unwrap();

        let promise = Promise::new(account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .deploy_contract(ESCROW_CODE.to_vec())
            .transfer(env::attached_deposit())
            .function_call(
                "new".to_string(),
                args.into(),
                0,
                env::prepaid_gas() - CREATE_CALL_GAS - ON_CREATE_CALL_GAS,
            );

        let callback = Promise::new(env::current_account_id())
            .function_call(
                "on_create_escrow".to_string(),
                json!({"account_id": account_id, "attached_deposit": U128(env::attached_deposit()), "predecessor_account_id": env::predecessor_account_id()})
                    .to_string()
                    .into_bytes(),
                0,
                ON_CREATE_CALL_GAS,
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_create_escrow(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool {
        assert_self();

        if near_sdk::is_promise_success() {
            self.escrows.insert(&account_id);
            true
        } else {
            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
            // @TODO, we need to panick to let the wallet notify the user, BUT we need to wait for the transfer Promise above to finish first
            env::panic_str("ERR_CREATE_ESCROW_UNSUCCESSFUL")
        }
    }

    /// Views

    pub fn get_escrows_list(&self) -> Vec<AccountId> {
        self.escrows.to_vec()
    }

    pub fn get_escrows_count(&self) -> u64 {
        self.escrows.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{serde_json::json, testing_env, AccountId};
    use near_sdk::PromiseResult;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn setup_contract() -> EscrowFactory {
        EscrowFactory::new()
    }

    fn factory_account_id() -> AccountId {
        AccountId::new_unchecked("factory.near".to_string())
    }

    #[test]
    fn create_escrow_success() {
        let mut context = get_context(factory_account_id());
        testing_env!(context.build());

        let mut contract = setup_contract();

        // Create Escrow
        let escrow1 = "sa1".to_string();

        contract.create_escrow(escrow1.clone(),
            json!({
                "args": "eyJtYXJrZ...=="
            }).to_string().into_bytes().to_vec().into());

        testing_env!(
            context
            .current_account_id(factory_account_id())
            .build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        let escrow1_account_id = AccountId::new_unchecked(escrow1.clone());
        let res = contract.on_create_escrow(escrow1_account_id.clone(), U128(1), env::predecessor_account_id());

        assert_eq!(res, true, "Escrow should be created successfully");

        // Create Escrow
        let escrow2 = "sa2".to_string();

        contract.create_escrow(escrow2.clone(),
            json!({
                "args": "eyJtYXJrZ...=="
            }).to_string().into_bytes().to_vec().into());

        testing_env!(
            context
            .current_account_id(factory_account_id())
            .build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        let escrow2_account_id = AccountId::new_unchecked(escrow2.clone());
        contract.on_create_escrow(escrow2_account_id.clone(), U128(1), env::predecessor_account_id());

        assert_eq!(
            contract.get_escrows_list(),
            vec![escrow1_account_id, escrow2_account_id],
            "sa1 and sa2 escrows should be listed"
        );

        assert_eq!(
            contract.get_escrows_count(),
            2,
            "There should be 2 escrows"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_CREATE_ESCROW_UNSUCCESSFUL")]
    fn create_escrow_fail() {
        let mut context = get_context(factory_account_id());
        testing_env!(context.build());

        let mut contract = setup_contract();

        // Create Escrow
        let escrow1 = "sa1".to_string();

        contract.create_escrow(escrow1.clone(),
            json!({
                "args": "eyJtYXJrZ...=="
            }).to_string().into_bytes().to_vec().into());

        testing_env!(
            context
            .current_account_id(factory_account_id())
            .build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Failed],
        );

        let escrow1_account_id = AccountId::new_unchecked(escrow1.clone());
        contract.on_create_escrow(escrow1_account_id.clone(), U128(1), env::predecessor_account_id());
    }
}
