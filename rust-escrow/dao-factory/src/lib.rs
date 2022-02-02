use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::Base64VecU8;
use near_sdk::{env, ext_contract, near_bindgen, Gas};
use near_sdk::{AccountId, Balance, Promise, PromiseResult};

/// Amount of gas for create action.
pub const GAS_FOR_CREATE: Gas = Gas(90_000_000_000_000);
pub const GAS_FOR_CREATE_CALLBACK: Gas = Gas(5_000_000_000_000);

// define the methods we'll use on the other contract
#[ext_contract(ext_dao_factory)]
pub trait ExtDaoFactory {
    fn create(&mut self, name: String, args: Base64VecU8);
}

// define methods we'll use as callbacks on our contract
#[ext_contract(ext_self)]
pub trait MyContract {
    fn on_create_callback(&mut self, country_code: String, daos_by_country: u128, predecessor_account_id: AccountId, attached_deposit: u128) -> bool;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct DaoFactory {
    daos_by_country_count: UnorderedMap<String, u128>, // Country code and sequential number
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
            daos_by_country_count: UnorderedMap::new(b"r".to_vec()),
            dao_factory_account,
        }
    }

    pub fn get_daos_by_country_count(&self, country_code: String) -> u128 {
        match self.daos_by_country_count.get(&country_code) {
            Some(count) => count,
            None => 0,
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
    pub fn create_dao(
        &mut self,
        country_code: String,
        deposits: Vec<(AccountId, Balance)>,
    ) -> Promise {
        let mut daos_by_country = self.get_daos_by_country_count(country_code.clone());
        daos_by_country = daos_by_country.wrapping_add(1);

        // Get Parameters
        let deposit_accounts = self.get_deposit_accounts(deposits);
        let dao_name = format!("ce_{}_{}", country_code.clone(), daos_by_country);
        let args = self.get_dao_config(dao_name.clone(), deposit_accounts);

        // Contract Call
        ext_dao_factory::create(
            dao_name.to_string(),
            Base64VecU8(args),
            self.dao_factory_account.clone(),
            env::attached_deposit(),
            GAS_FOR_CREATE,
        )
        .then(ext_self::on_create_callback(
            country_code,
            daos_by_country,
            env::predecessor_account_id(),
            env::attached_deposit(),
            env::current_account_id(),
            0,
            GAS_FOR_CREATE_CALLBACK,
        ))
    }

    #[private]
    pub fn on_create_callback(
        &mut self, 
        country_code: String, 
        daos_by_country: u128,
        predecessor_account_id: AccountId, 
        attached_deposit: u128
    ) -> bool {
        assert_eq!(env::promise_results_count(), 1, "ERR_CALLBACK_METHOD");

        // handle the result from the cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                let res = String::from_utf8_lossy(&result);
                if res == "true" {
                    self.daos_by_country_count.insert(&(country_code), &(daos_by_country));
                    return true;
                }
                Promise::new(predecessor_account_id).transfer(attached_deposit);
                false
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::test_env::{alice, bob};
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new()
            .signer_account_id(bob())
            .is_view(is_view)
            .build()
    }

    #[test]
    fn test_get_dao_config() {
        let context = get_context(false);
        testing_env!(context);

        let contract = DaoFactory::new("sputnikv2.testnet".parse::<AccountId>().unwrap());

        assert_eq!(
            contract.get_dao_config("daoname".to_string(), "poguz.testnet".to_string()),
            format!(r#"{{ "policy": {{ "roles": [ {{ "name": "Everyone", "kind": {{ "Group": [ {} ] }}, "permissions": [ "*:Finalize", "*:AddProposal", "*:VoteApprove", "*:VoteReject", "*:VoteRemove" ], "vote_policy": {{}} }}, {{ "name": "all", "kind": "Everyone", "permissions": [ "*:AddProposal" ], "vote_policy": {{}} }} ], "default_vote_policy": {{ "weight_kind": "RoleWeight", "quorum": "0", "threshold": [ 1, 2 ] }}, "proposal_bond": "100000000000000000000000", "proposal_period": "604800000000000", "bounty_bond": "100000000000000000000000", "bounty_forgiveness_period": "604800000000000" }}, "config": {{ "name": "{}", "purpose": "", "metadata": "" }} }}"#, "poguz.testnet".to_string(), "daoname".to_string()).into_bytes()
        );
    }

    #[test]
    fn test_get_daos_by_country_count() {
        let context = get_context(false);
        testing_env!(context);

        let contract = DaoFactory::new("sputnikv2.testnet".parse::<AccountId>().unwrap());

        assert_eq!(contract.get_daos_by_country_count("gt".to_string()), 0);
    }

    #[test]
    fn test_get_deposit_accounts() {
        let context = get_context(false);
        testing_env!(context);

        let contract = DaoFactory::new("sputnikv2.testnet".parse::<AccountId>().unwrap());

        assert_eq!(
            contract.get_deposit_accounts(vec![]),
            format!(r#""{}""#, alice().to_string().to_string())
        );

        assert_eq!(
            contract.get_deposit_accounts(vec![(bob(), 1000)]),
            format!(
                r#""{}", "{}""#,
                alice().to_string().to_string(),
                bob().to_string().to_string()
            )
        );
    }

    #[test]
    fn test_create_dao() {
        let context = get_context(false);
        testing_env!(context);

        let mut contract = DaoFactory::new("sputnikv2.testnet".parse::<AccountId>().unwrap());
        contract.create_dao("gt".to_string(), vec![]);
        contract.create_dao("gt".to_string(), vec![(bob(), 1000)]);
    }
}