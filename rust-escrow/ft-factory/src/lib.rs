use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::serde_json::json;
use near_sdk::{env, near_bindgen, Gas};
use near_sdk::{AccountId, Balance, Promise, PromiseResult};

// Fungile Token Contract
const FT_CODE: &[u8] = include_bytes!("../../src/fungible_token.wasm");

// Amount of gas used
pub const GAS_FOR_CREATE_FT: Gas = Gas(5_000_000_000_000);
pub const GAS_FOR_CREATE_FT_CB: Gas = Gas(5_000_000_000_000);

// Amount used for FT
pub const FT_SUPPLY: Balance = 100_000 * 10_000_000;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct FtFactory {
    ft_index: UnorderedMap<AccountId, AccountId>, // Escrow Account and FT Account
}

impl Default for FtFactory {
    fn default() -> Self {
        env::panic_str("FtFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl FtFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            ft_index: UnorderedMap::new(b"r".to_vec()),
        }
    }

    pub fn get_ft_by_escrow_account(&self, account: AccountId) -> String {
        match self.ft_index.get(&account) {
            Some(account_id) => account_id.to_string(),
            None => "".to_string(),
        }
    }

    #[payable]
    pub fn create_ft(&mut self, name: String) -> Promise {
        let ft_account_id: AccountId = format!("{}.{}", name, env::current_account_id())
            .parse()
            .unwrap();

        let promise = Promise::new(ft_account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(env::attached_deposit())
            .deploy_contract(FT_CODE.to_vec())
            .function_call(
                "new".to_string(),
                json!({"max_supply": FT_SUPPLY.to_string(), "escrow_account_id": env::predecessor_account_id().to_string(), "metadata": { "spec": "ft-1.0.0", "name": ft_account_id.to_string(), "symbol": "", "decimals": 8 }})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_CREATE_FT,
            );

        let callback = Promise::new(env::current_account_id()) // the recipient of this ActionReceipt (&self)
            .function_call(
                "on_create_ft_callback".to_string(), // the function call will be a callback function
                json!({"ft_account_id": ft_account_id.to_string()})
                    .to_string()
                    .into_bytes(), // method arguments
                0,                                   // amount of yoctoNEAR to attach
                GAS_FOR_CREATE_FT_CB,                // gas to attach
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_create_ft_callback(&mut self, ft_account_id: AccountId) {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.ft_index
                    .insert(&env::predecessor_account_id(), &ft_account_id);
            }
            _ => env::panic_str("ERR_CREATE_FT_UNSUCCESSFUL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::{alice, bob};
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::PublicKey;
    use near_sdk::{testing_env, PromiseResult};

    fn get_signer_pk() -> PublicKey {
        "ed25519:6E8sCci9badyRkXb3JoRpBj5p8C6Tw41ELDZoiihKEtp"
            .parse()
            .unwrap()
    }

    fn get_context() -> VMContextBuilder {
        let mut context = VMContextBuilder::new();
        testing_env!(context
            .predecessor_account_id(alice())
            .signer_account_id(bob())
            .build());

        context
    }

    fn get_contract() -> FtFactory {
        FtFactory::new()
    }

    #[test]
    fn test_get_ft_by_escrow_account() {
        let contract = get_contract();

        assert_eq!(
            contract.get_ft_by_escrow_account(bob()),
            "",
            "Ft's Bob should be empty"
        );
    }

    #[test]
    fn test_create_ft() {
        let mut context = get_context();
        let mut contract = get_contract();
        let signer_pk = get_signer_pk();

        // First Ft
        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .build());

        let ft_name = "ft1".to_string();
        contract.create_ft(ft_name.clone());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        let ft_account_id: AccountId = format!("{}.{}", ft_name.clone(), env::current_account_id())
            .parse()
            .unwrap();

        contract.on_create_ft_callback(ft_account_id.clone());

        assert_eq!(
            contract.get_ft_by_escrow_account(env::predecessor_account_id()),
            ft_account_id.to_string(),
            "A FT should be found"
        );

        // Second FT
        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(alice())
            .build());

        let ft_name = "ft2".to_string();

        contract.create_ft(ft_name.clone());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        let ft_account_id: AccountId = format!("{}.{}", ft_name.clone(), env::current_account_id())
            .parse()
            .unwrap();

        contract.on_create_ft_callback(ft_account_id.clone());

        assert_eq!(
            contract.get_ft_by_escrow_account(env::predecessor_account_id()),
            ft_account_id.to_string(),
            "A FT should be found"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_CREATE_FT_UNSUCCESSFUL")]
    fn test_create_ft_fail() {
        let mut context = get_context();
        let mut contract = get_contract();
        let signer_pk = get_signer_pk();

        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .build());

        let ft_name = "ft1".to_string();
        contract.create_ft(ft_name.clone());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Failed],
        );

        let ft_account_id: AccountId = format!("{}.{}", ft_name.clone(), env::current_account_id())
            .parse()
            .unwrap();

        contract.on_create_ft_callback(ft_account_id.clone());

        assert_eq!(
            contract.get_ft_by_escrow_account(env::predecessor_account_id()),
            "",
            "No FT should be found"
        );
    }
}
