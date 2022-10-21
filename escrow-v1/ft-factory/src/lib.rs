use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::U128;
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
        env::panic_str("ERR_FTFACTORY_NOT_INITIALIZED")
    }
}

#[near_bindgen]
impl FtFactory {
    #[init]
    pub fn new() -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }
        
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
        let predecessor_account_id = env::predecessor_account_id();
        let symbol = format!("NHA{}", self.ft_index.len() + 1);

        let promise = Promise::new(ft_account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(env::attached_deposit())
            .deploy_contract(FT_CODE.to_vec())
            .function_call(
                "new".to_string(),
                json!({"max_supply": FT_SUPPLY.to_string(), "escrow_account_id": predecessor_account_id, "metadata": { "spec": "ft-1.0.0", "name": name, "symbol": symbol, "decimals": 8 }})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_CREATE_FT,
            );

        let callback = Promise::new(env::current_account_id())
            .function_call(
                "on_create_ft_callback".to_string(),
                json!({"escrow_account_id": predecessor_account_id, "ft_account_id": ft_account_id.to_string(), "attached_deposit": U128(env::attached_deposit())})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_CREATE_FT_CB,
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_create_ft_callback(
        &mut self,
        escrow_account_id: AccountId,
        ft_account_id: AccountId,
        attached_deposit: U128,
    ) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.ft_index.insert(&escrow_account_id, &ft_account_id);
                true
            }
            _ => {
                Promise::new(escrow_account_id).transfer(attached_deposit.0);
                false
            }
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

        contract.on_create_ft_callback(
            env::predecessor_account_id(),
            ft_account_id.clone(),
            U128(1),
        );

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

        contract.on_create_ft_callback(
            env::predecessor_account_id(),
            ft_account_id.clone(),
            U128(1),
        );

        assert_eq!(
            contract.get_ft_by_escrow_account(env::predecessor_account_id()),
            ft_account_id.to_string(),
            "A FT should be found"
        );
    }

    #[test]
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

        assert_eq!(
            contract.on_create_ft_callback(
                env::predecessor_account_id(),
                ft_account_id.clone(),
                U128(1)
            ),
            false,
            "FT creation should be failed"
        );

        assert_eq!(
            contract.get_ft_by_escrow_account(env::predecessor_account_id()),
            "",
            "No FT should be found"
        );
    }
}
