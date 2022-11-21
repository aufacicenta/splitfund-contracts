use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::json_types::{Base64VecU8, U128};
use near_sdk::serde_json::json;
use near_sdk::{env, near_bindgen, Gas};
use near_sdk::{AccountId, Promise, PromiseResult};

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
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

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

    pub fn get_dao_factory_account(&self) -> AccountId {
        self.dao_factory_account.clone()
    }

    fn get_dao_config(&self, name: String, accounts: Vec<String>) -> Vec<u8> {
        json!({ "policy": { "roles": [ { "name": "Everyone", "kind": { "Group": accounts }, "permissions": [ "*:Finalize", "*:AddProposal", "*:VoteApprove", "*:VoteReject", "*:VoteRemove" ], "vote_policy": {} }, { "name": "all", "kind": "Everyone", "permissions": [ "*:AddProposal" ], "vote_policy": {} } ], "default_vote_policy": { "weight_kind": "RoleWeight", "quorum": "0", "threshold": [ 1, 2 ] }, "proposal_bond": "100000000000000000000000", "proposal_period": "604800000000000", "bounty_bond": "100000000000000000000000", "bounty_forgiveness_period": "604800000000000" }, "config": { "name": name, "purpose": "", "metadata": "" } })
            .to_string()
            .into_bytes()
    }

    #[payable]
    pub fn create_dao(&mut self, dao_name: String, deposits: Vec<String>) -> Promise {
        let args = self.get_dao_config(dao_name.clone(), deposits);
        let predecessor_account_id = env::predecessor_account_id();

        let promise = Promise::new(self.dao_factory_account.clone()).function_call(
            "create".to_string(),
            json!({"name": dao_name.clone(), "args": Base64VecU8(args) })
                .to_string()
                .into_bytes(),
            env::attached_deposit(),
            GAS_FOR_CREATE_DAO,
        );

        let callback = Promise::new(env::current_account_id())
            .function_call(
                "on_create_dao_callback".to_string(),
                json!({"escrow_account_id": predecessor_account_id, "dao_name": dao_name.clone(), "attached_deposit": U128(env::attached_deposit())})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_CREATE_DAO_CB,
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_create_dao_callback(
        &mut self,
        escrow_account_id: AccountId,
        dao_name: String,
        attached_deposit: U128,
    ) -> bool {
        if env::promise_results_count() != 1 {
            env::panic_str("ERR_CALLBACK_METHOD");
        }

        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                let res: bool = near_sdk::serde_json::from_slice(&result).unwrap();

                if res {
                    let dao_account_id: AccountId =
                        format!("{}.{}", dao_name, self.dao_factory_account)
                            .parse()
                            .unwrap();
                    self.dao_index.insert(&escrow_account_id, &dao_account_id);
                    return true;
                } else {
                    Promise::new(escrow_account_id).transfer(attached_deposit.0);
                    return false;
                }
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
    use near_sdk::{testing_env, Balance, PromiseResult};

    pub const ATTACHED_DEPOSIT: Balance = 10_000_000_000_000_000_000_000_000; // 10 NEAR
    const DAO_FACTORY_ACCOUNT: &str = "sputnikv2.testnet";

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
        DaoFactory::new(DAO_FACTORY_ACCOUNT.parse::<AccountId>().unwrap())
    }

    #[test]
    fn test_get_dao_config() {
        let contract = get_contract();

        assert_eq!(
            contract.get_dao_config("daoname".to_string(), vec!["poguz.testnet".to_string()]),
            json!({ "policy": { "roles": [ { "name": "Everyone", "kind": { "Group": vec!["poguz.testnet".to_string()] }, "permissions": [ "*:Finalize", "*:AddProposal", "*:VoteApprove", "*:VoteReject", "*:VoteRemove" ], "vote_policy": {} }, { "name": "all", "kind": "Everyone", "permissions": [ "*:AddProposal" ], "vote_policy": {} } ], "default_vote_policy": { "weight_kind": "RoleWeight", "quorum": "0", "threshold": [ 1, 2 ] }, "proposal_bond": "100000000000000000000000", "proposal_period": "604800000000000", "bounty_bond": "100000000000000000000000", "bounty_forgiveness_period": "604800000000000" }, "config": { "name": "daoname".to_string(), "purpose": "", "metadata": "" } })
                .to_string()
                .into_bytes()
        );
    }

    #[test]
    fn test_get_dao_factory_account() {
        let contract = get_contract();

        assert_eq!(
            contract.get_dao_factory_account(),
            DAO_FACTORY_ACCOUNT.parse::<AccountId>().unwrap(),
            "Should equal DAO Factory account"
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

        let dao_account_id = format!("{}.{}", dao_name.clone(), DAO_FACTORY_ACCOUNT);
        contract.on_create_dao_callback(env::predecessor_account_id(), dao_name.clone(), U128(1));

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
        contract.create_dao(dao_name.clone(), vec![bob().to_string()]);

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("true".to_string().into_bytes())],
        );

        let dao_account_id = format!("{}.{}", dao_name.clone(), DAO_FACTORY_ACCOUNT);

        contract.on_create_dao_callback(env::predecessor_account_id(), dao_name.clone(), U128(1));
        assert_eq!(
            contract.get_dao_by_escrow_account(env::predecessor_account_id()),
            dao_account_id,
            "A DAO should be found"
        );
    }

    #[test]
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
        contract.create_dao(dao_name.clone(), vec![bob().to_string()]);

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("false".to_string().into_bytes())],
        );

        assert_eq!(
            contract.on_create_dao_callback(
                env::predecessor_account_id(),
                dao_name.clone(),
                U128(1)
            ),
            false,
            "DAO creation should fail"
        );

        assert_eq!(
            contract.get_dao_by_escrow_account(env::predecessor_account_id()),
            "",
            "No DAO should be found"
        );
    }
}
