use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{U128, U64};
use near_sdk::serde_json::json;
use near_sdk::{env, near_bindgen, Gas};
use near_sdk::{AccountId, Promise, PromiseResult};

// Staking Contract
const STAKING_CODE: &[u8] = include_bytes!("../../src/staking.wasm");

// Amount of gas used
pub const GAS_FOR_CREATE_SK: Gas = Gas(5_000_000_000_000);
pub const GAS_FOR_CREATE_SK_CB: Gas = Gas(5_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct StakingFactory {
    staking_index: UnorderedMap<AccountId, AccountId>, // Escrow Account and Stacking Account
}

impl Default for StakingFactory {
    fn default() -> Self {
        env::panic_str("ERR_STAKING_NOT_INITIALIZED")
    }
}

#[near_bindgen]
impl StakingFactory {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "ERR_CONTRACT_ALREADY_INITIALIZED");
        Self {
            staking_index: UnorderedMap::new(b"r".to_vec()),
        }
    }

    pub fn get_staking_by_escrow_account(&self, account: AccountId) -> String {
        match self.staking_index.get(&account) {
            Some(account_id) => account_id.to_string(),
            None => "".to_string(),
        }
    }

    #[payable]
    pub fn create_stake(&mut self, name: String, dao_account_id: AccountId, token_account_id: AccountId, unstake_period: U64) -> Promise {
        let stake_account_id: AccountId = format!("{}.{}", name, env::current_account_id())
            .parse()
            .unwrap();
        let predecessor_account_id = env::predecessor_account_id();

        let promise = Promise::new(stake_account_id.clone())
            .create_account()
            .add_full_access_key(env::signer_account_pk())
            .transfer(env::attached_deposit())
            .deploy_contract(STAKING_CODE.to_vec())
            .function_call(
                "new".to_string(),
                json!({"owner_id": dao_account_id, "token_id": token_account_id, "unstake_period": unstake_period})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_CREATE_SK,
            );

        let callback = Promise::new(env::current_account_id())
            .function_call(
                "on_create_stake_callback".to_string(),
                json!({"escrow_account_id": predecessor_account_id, "stake_account_id": stake_account_id.to_string(), "attached_deposit": U128(env::attached_deposit())})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_CREATE_SK_CB,
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_create_stake_callback(&mut self, escrow_account_id: AccountId, stake_account_id: AccountId, attached_deposit: U128) -> bool {
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.staking_index
                    .insert(&escrow_account_id, &stake_account_id);
                true
            }
            _ => {
                Promise::new(escrow_account_id).transfer(attached_deposit.0);
                false
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::{alice, bob};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
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

    fn get_contract() -> StakingFactory {
        StakingFactory::new()
    }

    #[test]
    fn test_get_staking_by_escrow_account() {
        let contract = get_contract();

        assert_eq!(
            contract.get_staking_by_escrow_account(bob()),
            "",
            "Staking's Bob should be empty"
        );
    }

    #[test]
    fn test_create_stake() {
        let mut context = get_context();
        let mut contract = get_contract();
        let signer_pk = get_signer_pk();

        // First Staking
        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .build());

        let staking_name = "sk1".to_string();
        contract.create_stake(staking_name.clone(), accounts(1), accounts(2), U64(1000));

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        let stake_account_id: AccountId = format!("{}.{}", staking_name.clone(), env::current_account_id())
            .parse()
            .unwrap();

        contract.on_create_stake_callback(env::predecessor_account_id(), stake_account_id.clone(), U128(1));

        assert_eq!(
            contract.get_staking_by_escrow_account(env::predecessor_account_id()),
            stake_account_id.to_string(),
            "A Staking should be found"
        );

        // Second Staking
        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(alice())
            .build());

        let staking_name = "sk2".to_string();

        contract.create_stake(staking_name.clone(), accounts(1), accounts(2), U64(1000));

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        let stake_account_id: AccountId = format!("{}.{}", staking_name.clone(), env::current_account_id())
            .parse()
            .unwrap();

        contract.on_create_stake_callback(env::predecessor_account_id(), stake_account_id.clone(), U128(1));

        assert_eq!(
            contract.get_staking_by_escrow_account(env::predecessor_account_id()),
            stake_account_id.to_string(),
            "A Staking should be found"
        );
    }

    #[test]
    fn test_create_stake_fail() {
        let mut context = get_context();
        let mut contract = get_contract();
        let signer_pk = get_signer_pk();

        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .build());

        let staking_name = "sk1".to_string();
        contract.create_stake(staking_name.clone(), accounts(1), accounts(2), U64(1000));

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Failed],
        );

        let stake_account_id: AccountId = format!("{}.{}", staking_name.clone(), env::current_account_id())
            .parse()
            .unwrap();

        assert_eq!(
            contract.on_create_stake_callback(env::predecessor_account_id(), stake_account_id.clone(), U128(1)),
            false,
            "Staking creation should be failed"
        );

        assert_eq!(
            contract.get_staking_by_escrow_account(env::predecessor_account_id()),
            "",
            "No Staking should be found"
        );
    }
}
