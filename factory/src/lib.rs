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

const ESCROW_CODE: &[u8] = include_bytes!("./escrow.wasm");

/// Gas spent on the call & account creation.
const CREATE_CALL_GAS: Gas = Gas(75_000_000_000_000);

/// Gas allocated on the callback.
const ON_CREATE_CALL_GAS: Gas = Gas(10_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct EscrowFactory {
    conditional_escrow_contracts: UnorderedSet<AccountId>,
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
            conditional_escrow_contracts: UnorderedSet::new(b"d".to_vec()),
        }
    }

    #[payable]
    pub fn create_conditional_escrow(&mut self, name: AccountId, args: Base64VecU8) -> Promise {
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
                "on_create_conditional_escrow".to_string(),
                json!({"account_id": account_id, "attached_deposit": U128(env::attached_deposit()), "predecessor_account_id": env::predecessor_account_id()})
                    .to_string()
                    .into_bytes(),
                0,
                ON_CREATE_CALL_GAS,
            );

        promise.then(callback)
    }

    pub fn on_create_conditional_escrow(
        &mut self,
        account_id: AccountId,
        attached_deposit: U128,
        predecessor_account_id: AccountId,
    ) -> bool {
        assert_self();

        if near_sdk::is_promise_success() {
            self.conditional_escrow_contracts.insert(&account_id);
            true
        } else {
            Promise::new(predecessor_account_id).transfer(attached_deposit.0);
            // @TODO, we need to panick to let the wallet notify the user, BUT we need to wait for the transfer Promise above to finish first
            env::panic_str("ERR_CREATE_ESCROW_UNSUCCESSFUL")
        }
    }
}
