use near_sdk::{
    collections::{LazyOption, UnorderedSet},
    env,
    json_types::{Base64VecU8, U128},
    near_bindgen,
    serde_json::json,
    AccountId, Balance, Promise, PromiseOrValue,
};

use near_contract_standards::fungible_token::{
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider},
    FungibleToken,
};

use crate::consts::*;
use crate::storage::*;

impl Default for Escrow {
    fn default() -> Self {
        env::panic_str("Escrow Contract should be initialized before usage")
    }
}

#[near_bindgen]
impl Escrow {
    #[init]
    pub fn new(
        decimals: u8,
        expires_at: u64,
        funding_amount_limit: U128,
        id: String,
        nep_141: AccountId,
        dao_factory: AccountId,
        maintainer: AccountId,
        metadata_url: String,
        staking_factory: AccountId,
    ) -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

        let ft_metadata = FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: id.clone(),
            symbol: id.clone(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals,
        };

        let mut token = FungibleToken::new(b"t".to_vec());
        token.total_supply = funding_amount_limit.0;

        Self {
            deposits: UnorderedSet::new(b"d".to_vec()),
            ft: token,
            ft_metadata: LazyOption::new(b"m".to_vec(), Some(&ft_metadata)),
            metadata: Metadata {
                expires_at,
                funding_amount_limit: funding_amount_limit.0,
                id,
                unpaid_amount: funding_amount_limit.0,
                nep_141,
                dao_factory,
                maintainer,
                metadata_url,
                staking_factory,
                dao_created: false,
                stake_created: false,
            },
        }
    }

    /**
     * Called on ft_transfer_callback only
     * Contract NEP141 balance is kept in the NEP141 contract
     * Sender balances are kept in the balances map
     * Total sender balances must match the contract NEP141 balance, minus fees
     *
     * Transfer self NEP141 in exchange of the stable NEP141 amount as a receipt
     */
    #[private]
    pub fn deposit(&mut self, sender_id: AccountId, amount: Balance) {
        if !self.is_deposit_allowed() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        if amount > self.get_unpaid_amount() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        self.ft.internal_deposit(&sender_id, amount);
        self.deposits.insert(&sender_id);
        self.metadata.unpaid_amount = self
            .metadata
            .unpaid_amount
            .checked_sub(amount)
            .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

        // @TODO log
    }

    /**
     * Transfer funds from contract NEP141 balance to sender_id
     */
    #[payable]
    pub fn withdraw(&mut self) -> Promise {
        if !self.is_withdrawal_allowed() {
            env::panic_str("ERR_WITHDRAWAL_NOT_ALLOWED");
        }

        let payee = env::signer_account_id();
        let balance = self.ft.internal_unwrap_balance_of(&payee);

        // Transfer from collateral token to payee
        let promise = Promise::new(self.metadata.nep_141.clone()).function_call(
            "ft_transfer".to_string(),
            json!({
                "amount": balance.to_string(),
                "receiver_id": payee.to_string(),
            })
            .to_string()
            .into_bytes(),
            1, // 1 yoctoNEAR
            GAS_FT_TRANSFER,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_withdraw_callback".to_string(),
            json!({
                "payee": payee.to_string(),
                "balance": balance.to_string(),
            })
            .to_string()
            .into_bytes(),
            0,
            GAS_FT_WITHDRAW_CALLBACK,
        );

        promise.then(callback)
    }

    /**
     * Only if total funds are reached, allow to call this function
     * Transfer total NEP141 funds to a new DAO
     * Make the depositors members of the DAO
     */
    #[payable]
    pub fn _delegate_funds(&mut self) {
        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_DELEGATE_NOT_ALLOWED");
        }

        // env::panic_str("ERR_TOTAL_FUNDS_OVERFLOW");

        // @TODO charge a fee here (1.5% initially?) when a property is sold by our contract

        // @TODO transfer the NEP141 stable coin funds to the DAO
    }

    #[payable]
    pub fn create_dao(&mut self) -> Promise {
        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_DELEGATE_NOT_ALLOWED");
        }

        if self.is_dao_created() {
            env::panic_str("ERR_DAO_ALREADY_CREATED");
        }

        let args = self.get_dao_config(self.metadata.id.clone(),
            vec![
                self.metadata.maintainer.to_string(),
                env::current_account_id().to_string(),
            ],
            self.get_deposit_accounts()
        );

        let promise = Promise::new(self.get_dao_factory_account_id()).function_call(
            "create".to_string(),
            json!({ "name": self.metadata.id, "args": Base64VecU8(args) })
                .to_string()
                .into_bytes(),
            env::attached_deposit(),
            GAS_FOR_CREATE_DAO,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_create_dao_callback".to_string(),
            json!({}).to_string().into_bytes(),
            0,
            GAS_FOR_CREATE_DAO_CB,
        );

        promise.then(callback)
    }

    #[payable]
    pub fn create_stake(&mut self) -> Promise {
        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_DELEGATE_NOT_ALLOWED");
        }

        if !self.is_dao_created() {
            env::panic_str("ERR_DAO_IS_NOT_CREATED");
        }

        if self.is_stake_created() {
            env::panic_str("ERR_STAKE_ALREADY_CREATED");
        }

        let promise = Promise::new(self.metadata.staking_factory.clone()).function_call(
            "create_stake".to_string(),
            json!({
                "name": self.metadata.id,
                "dao_account_id": format!("{}.{}", self.metadata.id, self.metadata.dao_factory),
                "token_account_id": self.metadata.nep_141,
                "unstake_period": PROPOSAL_PERIOD.to_string(),
            })
            .to_string()
            .into_bytes(),
            env::attached_deposit(),
            GAS_FOR_CREATE_STAKE,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_create_stake_callback".to_string(),
            json!({}).to_string().into_bytes(),
            0,
            GAS_FOR_CREATE_STAKE_CB,
        );

        promise.then(callback)
    }
}

impl Escrow {
    fn get_dao_config(&self, name: String, council: Vec<String>, investors: Vec<String>) -> Vec<u8> {
        json!({
            "policy": {
                "roles": [
                    {
                        "name": "council",
                        "kind": { "Group": council },
                        "permissions": [ "*:*" ],
                        "vote_policy": {}
                    },
                    {
                        "name": "investors",
                        "kind": { "Group": investors },
                        "permissions": [ "*:*" ], //@TODO Which permissions will the investors have in the DAO ?
                        "vote_policy": {
                            "*": {
                                "weight_kind": "TokenWeight",
                                "quorum": "0",
                                "threshold": [ 1, 2 ]
                            }
                        }
                    },
                    {
                        "name": "all",
                        "kind": "Everyone",
                        "permissions": [ "*:AddProposal" ],
                        "vote_policy": {}
                    }
                ],
                "default_vote_policy": {
                    "weight_kind": "RoleWeight",
                    "quorum": "0",
                    "threshold": [ 1, 2 ]
                },
                "proposal_bond": "100000000000000000000000",
                "proposal_period": PROPOSAL_PERIOD.to_string(),
                "bounty_bond": "100000000000000000000000",
                "bounty_forgiveness_period": PROPOSAL_PERIOD.to_string()
            },
            "config": {
                "name": name,
                "purpose": "",
                "metadata": ""
            }
        })
        .to_string()
        .into_bytes()
    }
}

near_contract_standards::impl_fungible_token_core!(Escrow, ft);
near_contract_standards::impl_fungible_token_storage!(Escrow, ft);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Escrow {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.ft_metadata.get().unwrap()
    }
}
