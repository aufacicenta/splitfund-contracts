use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, ext_contract, Gas, log, near_bindgen};
use near_sdk::json_types::{Base64VecU8};
use near_sdk::{AccountId, Balance, Promise, PromiseResult};

/// Amount of gas for create action.
pub const GAS_FOR_CREATE: Gas = Gas(90_000_000_000_000);
pub const GAS_FOR_CREATE_CALLBACK: Gas = Gas(5_000_000_000_000);

// define the methods we'll use on the other contract
#[ext_contract(ext_dao_factory)]
pub trait DaoFactory {
    fn create(&mut self, name: String, args: Base64VecU8);
}

// define methods we'll use as callbacks on our contract
#[ext_contract(ext_self)]
pub trait MyContract {
    fn on_create_callback(&mut self, country_code: String, daos_by_country: u128) -> bool;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct DaoEscrow {
    daos: UnorderedMap<String, u128>, // Country code and sequential number
    dao_factory_account: AccountId
}

impl Default for DaoEscrow {
    fn default() -> Self {
        env::panic_str("DaoEscrow should be initialized before usage")
    }
}

#[near_bindgen]
impl DaoEscrow {
    #[init]
    pub fn new(dao_factory_account: AccountId) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            daos: UnorderedMap::new(b"r".to_vec()),
            dao_factory_account: dao_factory_account,
        }
    }

    pub fn daos_of(&self, country_code: &String) -> u128 {
        match self.daos.get(country_code) {
            Some(country) => country,
            None => 0,
        }
    }

    fn generate_dao_config(&self, name: &String, purpose: &String) -> Vec<u8> {
        format!(r#"{{ "policy": {{ "roles": [ {{ "name": "Everyone", "kind": {{ "Group": [ "poguz.testnet" ] }}, "permissions": [ "*:Finalize", "*:AddProposal", "*:VoteApprove", "*:VoteReject", "*:VoteRemove" ], "vote_policy": {{}} }}, {{ "name": "all", "kind": "Everyone", "permissions": [ "*:AddProposal" ], "vote_policy": {{}} }} ], "default_vote_policy": {{ "weight_kind": "RoleWeight", "quorum": "0", "threshold": [ 1, 2 ] }}, "proposal_bond": "100000000000000000000000", "proposal_period": "604800000000000", "bounty_bond": "100000000000000000000000", "bounty_forgiveness_period": "604800000000000" }}, "config": {{ "name": "{}", "purpose": "{}", "metadata": "" }} }}"#, name, purpose).into_bytes()
    }

    #[payable]
    pub fn create_dao_call(&mut self, country_code: String, escrow_account: AccountId) -> Promise {
        let mut daos_by_country = self.daos_of(&country_code);
        daos_by_country = daos_by_country.wrapping_add(1);
        
        let dao_name = format!("ce_{}_{}", country_code, daos_by_country);
        let args = self.generate_dao_config(&dao_name, &"".to_string());

        // Contract Call
        ext_dao_factory::create(
            dao_name.to_string(),
            Base64VecU8(args),
            self.dao_factory_account.clone(),
            env::attached_deposit(),
            GAS_FOR_CREATE,
        ).then(ext_self::on_create_callback(
            country_code,
            daos_by_country,
            env::current_account_id(),
            0,
            GAS_FOR_CREATE_CALLBACK,
        ))
        //log!("{} daos en el pais {}, escrow {}", daos_by_country, country_code, escrow_account);
        //let config = self.generate_dao_config("hola".to_string(), "nada".to_string());
        //log!("{:?}", config);
    }

    pub fn on_create_callback(&mut self, country_code: String, daos_by_country: u128) -> bool{
        assert_eq!(
            env::promise_results_count(),
            1,
            "This is a callback method"
        );

        // handle the result from the cross contract call this method is a callback for
        match env::promise_result(0) {
            PromiseResult::Successful(_result) => {
                self.daos.insert(&(country_code), &(daos_by_country));
                true
            }
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};
    use near_sdk::test_utils::test_env::{bob};

    fn get_context(is_view: bool) -> VMContext {
        VMContextBuilder::new()
            .signer_account_id(bob())
            .is_view(is_view)
            .build()
    }

    #[test]
    fn test_create_dao() {
        let context = get_context(false);
        testing_env!(context);

        let mut contract = DaoEscrow::new("sputnikv2.testnet".parse::<AccountId>().unwrap());
        contract.create_dao_call("gt".to_string(), bob());
        contract.create_dao_call("gt".to_string(), bob());

        //assert_eq!(
        //    contract.daos_of(&"gt".to_string()),
        //    2
        //);
    }
}
