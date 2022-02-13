use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::Base64VecU8;
use near_sdk::serde_json::json;
use near_sdk::{env, near_bindgen, Gas};
use near_sdk::{AccountId, Balance, Promise, PromiseResult};

// Amount of gas used
pub const GAS_FOR_CREATE_DAO: Gas = Gas(90_000_000_000_000);
pub const GAS_FOR_CREATE_DAO_CB: Gas = Gas(5_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct DaoFactory {
    dao_index: UnorderedMap<AccountId, AccountId>, // Escrow Account and Dao Account
    dao_factory_account: AccountId,
}

impl Default for DaoFactory {
    fn default() -> Self {
        env::panic_str("DaoFactory should be initialized before usage")
    }
}

#[near_bindgen]
impl DaoFactory {
    #[init]
    pub fn new(dao_factory_account: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            dao_index: UnorderedMap::new(b"r".to_vec()),
            dao_factory_account,
        }
    }

    pub fn get_dao_by_escrow_account(&self, account: AccountId) -> String {
        match self.dao_index.get(&account) {
            Some(account_id) => account_id.to_string(),
            None => "".to_string(),
        }
    }

    fn get_dao_config(&self, name: String, accounts: String) -> Vec<u8> {
        format!(r#"{{ "policy": {{ "roles": [ {{ "name": "Everyone", "kind": {{ "Group": [ {} ] }}, "permissions": [ "*:Finalize", "*:AddProposal", "*:VoteApprove", "*:VoteReject", "*:VoteRemove" ], "vote_policy": {{}} }}, {{ "name": "all", "kind": "Everyone", "permissions": [ "*:AddProposal" ], "vote_policy": {{}} }} ], "default_vote_policy": {{ "weight_kind": "RoleWeight", "quorum": "0", "threshold": [ 1, 2 ] }}, "proposal_bond": "100000000000000000000000", "proposal_period": "604800000000000", "bounty_bond": "100000000000000000000000", "bounty_forgiveness_period": "604800000000000" }}, "config": {{ "name": "{}", "purpose": "", "metadata": "" }} }}"#, accounts, name).into_bytes()
    }

    fn get_deposit_accounts(&self, deposits: Vec<(AccountId, Balance)>) -> String {
        let mut accounts = format!(r#""{}""#, env::current_account_id().to_string());

        for i in &deposits {
            accounts += ", ";
            accounts += &format!(r#""{}""#, &i.0.to_string().to_string());
        }

        accounts
    }

    #[payable]
    pub fn create_dao(&mut self, dao_name: String, deposits: Vec<(AccountId, Balance)>) -> Promise {
        let deposit_accounts = self.get_deposit_accounts(deposits);
        let args = self.get_dao_config(dao_name.clone(), deposit_accounts);

        let promise = Promise::new(self.dao_factory_account.clone()).function_call(
            "create".to_string(),
            json!({"name": dao_name.clone(), "args": Base64VecU8(args) })
                .to_string()
                .into_bytes(),
            env::attached_deposit(),
            GAS_FOR_CREATE_DAO,
        );

        let callback = Promise::new(env::current_account_id()) // the recipient of this ActionReceipt (&self)
            .function_call(
                "on_create_dao_callback".to_string(), // the function call will be a callback function
                json!({"dao_name": dao_name.clone()})
                    .to_string()
                    .into_bytes(), // method arguments
                0,                                    // amount of yoctoNEAR to attach
                GAS_FOR_CREATE_DAO_CB,                // gas to attach
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_create_dao_callback(&mut self, dao_name: String) {
        assert_eq!(env::promise_results_count(), 1, "ERR_CALLBACK_METHOD");

        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                let res: bool = near_sdk::serde_json::from_slice(&result).unwrap();

                if res {
                    let dao_account_id: AccountId =
                        format!("{}.{}", dao_name, self.dao_factory_account)
                            .parse()
                            .unwrap();
                    self.dao_index
                        .insert(&env::predecessor_account_id(), &dao_account_id);
                } else {
                    Promise::new(env::predecessor_account_id()).transfer(env::attached_deposit());
                    panic!("ERR_CREATE_DAO_UNSUCCESSFUL");
                }
            }
            _ => {
                Promise::new(env::predecessor_account_id()).transfer(env::attached_deposit());
                panic!("ERR_CREATE_DAO_UNSUCCESSFUL");
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

    pub const ATTACHED_DEPOSIT: Balance = 10_000_000_000_000_000_000_000_000; // 10 NEAR

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

    fn get_contract() -> DaoFactory {
        DaoFactory::new("sputnikv2.testnet".parse::<AccountId>().unwrap())
    }

    #[test]
    fn test_get_dao_config() {
        let contract = get_contract();

        assert_eq!(
            contract.get_dao_config("daoname".to_string(), "poguz.testnet".to_string()),
            format!(r#"{{ "policy": {{ "roles": [ {{ "name": "Everyone", "kind": {{ "Group": [ {} ] }}, "permissions": [ "*:Finalize", "*:AddProposal", "*:VoteApprove", "*:VoteReject", "*:VoteRemove" ], "vote_policy": {{}} }}, {{ "name": "all", "kind": "Everyone", "permissions": [ "*:AddProposal" ], "vote_policy": {{}} }} ], "default_vote_policy": {{ "weight_kind": "RoleWeight", "quorum": "0", "threshold": [ 1, 2 ] }}, "proposal_bond": "100000000000000000000000", "proposal_period": "604800000000000", "bounty_bond": "100000000000000000000000", "bounty_forgiveness_period": "604800000000000" }}, "config": {{ "name": "{}", "purpose": "", "metadata": "" }} }}"#, "poguz.testnet".to_string(), "daoname".to_string()).into_bytes()
        );
    }

    #[test]
    fn test_get_daos_by_country_count() {
        let contract = get_contract();

        assert_eq!(
            contract.get_dao_by_escrow_account(bob()),
            "",
            "DAO's Bob should be empty"
        );
    }

    #[test]
    fn test_get_deposit_accounts() {
        let contract = get_contract();

        assert_eq!(
            contract.get_deposit_accounts(vec![]),
            format!(r#""{}""#, alice().to_string().to_string()),
            "The Depositor should be alice"
        );

        assert_eq!(
            contract.get_deposit_accounts(vec![(bob(), 1000)]),
            format!(
                r#""{}", "{}""#,
                alice().to_string().to_string(),
                bob().to_string().to_string()
            ),
            "Depositors should be alice and bob"
        );
    }

    #[test]
    fn test_create_dao() {
        let mut context = get_context();
        let mut contract = get_contract();
        let signer_pk = get_signer_pk();

        // First DAO
        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let dao_name = "dao1".to_string();
        contract.create_dao(dao_name.clone(), vec![]);

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("true".to_string().into_bytes())],
        );

        let dao_account_id = format!("{}.{}", dao_name.clone(), "sputnikv2.testnet".to_string());
        contract.on_create_dao_callback(dao_name.clone());

        assert_eq!(
            contract.get_dao_by_escrow_account(env::predecessor_account_id()),
            dao_account_id,
            "A DAO should be found"
        );

        // Second DAO
        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let dao_name = "dao2".to_string();
        contract.create_dao(dao_name.clone(), vec![(bob(), 1000)]);

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("true".to_string().into_bytes())],
        );

        let dao_account_id = format!("{}.{}", dao_name.clone(), "sputnikv2.testnet".to_string());

        contract.on_create_dao_callback(dao_name.clone());
        assert_eq!(
            contract.get_dao_by_escrow_account(env::predecessor_account_id()),
            dao_account_id,
            "A DAO should be found"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_CREATE_DAO_UNSUCCESSFUL")]
    fn test_create_dao_fail() {
        let mut context = get_context();
        let mut contract = get_contract();
        let signer_pk = get_signer_pk();

        testing_env!(context
            .signer_account_pk(signer_pk.clone())
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let dao_name = "dao1".to_string();
        contract.create_dao(dao_name.clone(), vec![(bob(), 1000)]);

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("false".to_string().into_bytes())],
        );

        contract.on_create_dao_callback(dao_name.clone());

        assert_eq!(
            contract.get_dao_by_escrow_account(env::predecessor_account_id()),
            "",
            "No DAO should be found"
        );
    }
}
