use near_sdk::{
    collections::{LazyOption, UnorderedSet},
    env, log,
    json_types::{Base64VecU8, U128},
    near_bindgen,
    serde_json::{json, Value},
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
        fee_percentage: f32
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
        token.internal_register_account(&maintainer);

        let mut deposits = UnorderedSet::new(b"d".to_vec());
        deposits.insert(&maintainer);

        Self {
            deposits,
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
                dao_setuped: false,
                stake_created: false,
                fee_percentage,
                fee_balance: 0,
            },
        }
    }

    /**
     * Only if total funds are reached or escrow has not expired
     * Called on ft_transfer_callback only
     * Total sender balances must match the contract NEP141 balance, minus fees
     * Transfer self NEP141 of the stable NEP141 amount as a receipt
     */
    #[private]
    pub fn deposit(&mut self, sender_id: AccountId, amount: Balance) {
        if !self.is_deposit_allowed() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        if amount > self.get_unpaid_amount() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        // Fee Calculations
        let fee_amount = (amount as f32 * self.metadata.fee_percentage) as Balance;
        self.metadata.fee_balance = self.metadata.fee_balance
            .checked_add(fee_amount)
            .unwrap_or_else(|| env::panic_str("ERR_FEE_BALANCE_OVERFLOW"));
        let amount_fee = amount.checked_sub(fee_amount)
            .unwrap_or_else(|| env::panic_str("ERR_AMOUNT_OVERFLOW"));

        // Register transfer
        self.ft.internal_deposit(&sender_id, amount_fee);
        self.deposits.insert(&sender_id);
        self.metadata.unpaid_amount = self
            .metadata
            .unpaid_amount
            .checked_sub(amount)
            .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

        log!("Successful Deposit. Account: {}, Amount: {}, Fee: {}", sender_id, amount_fee, fee_amount);
    }

    /**
     * Only if total funds are not reached or escrow has expired
     * Transfer all funds to receiver_id
     */
    #[payable]
    pub fn withdraw(&mut self) -> Promise {
        if !self.is_withdrawal_allowed() {
            env::panic_str("ERR_WITHDRAWAL_NOT_ALLOWED");
        }

        let receiver_id = env::signer_account_id();
        let amount = self.ft.internal_unwrap_balance_of(&receiver_id);

        let promise = Promise::new(self.metadata.nep_141.clone()).function_call(
            "ft_transfer".to_string(),
            json!({
                "amount": amount.to_string(),
                "receiver_id": receiver_id.to_string(),
            })
            .to_string()
            .into_bytes(),
            1, // 1 yoctoNEAR
            GAS_FT_TRANSFER,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_withdraw_callback".to_string(),
            json!({
                "receiver_id": receiver_id.to_string(),
                "amount": amount.to_string(),
            })
            .to_string()
            .into_bytes(),
            0,
            GAS_FT_TRANSFER_CB,
        );

        promise.then(callback)
    }

    #[payable]
    pub fn claim_fees(&mut self) -> Promise {
        if !self.is_withdrawal_allowed() {
            if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
                env::panic_str("ERR_CLAIM_FEES_NOT_ALLOWED");
            }
        }

        if self.metadata.fee_balance == 0 {
            env::panic_str("ERR_FEES_ALREADY_CLAIMED");
        }

        let mut amount = self.metadata.fee_balance;

        if !self.is_deposit_allowed() && !self.is_withdrawal_allowed() {
            // If the escrow is successful
            // Half of the fees are collected and half invested in the asset
            amount = (self.metadata.fee_balance as f32 * 0.5) as Balance;
        }

        let receiver_id = self.metadata.maintainer.clone();

        let promise = Promise::new(self.metadata.nep_141.clone()).function_call(
            "ft_transfer".to_string(),
            json!({
                "amount": amount.to_string(),
                "receiver_id": receiver_id.to_string(),
            })
            .to_string()
            .into_bytes(),
            1, // 1 yoctoNEAR
            GAS_FT_TRANSFER,
        );

        amount = self.metadata.fee_balance
            .checked_sub(amount)
            .unwrap_or_else(|| env::panic_str("ERR_FEE_BALANCE_OVERFLOW"));

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_claim_fees_callback".to_string(),
            json!({
                "amount": amount.to_string(),
            })
            .to_string()
            .into_bytes(),
            0,
            GAS_FT_TRANSFER_CB,
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

    /**
     * Only if total funds are reached and if dao it not created
     * Create a new sputnik dao with this contract account in the council
     */
    #[payable]
    pub fn create_dao(&mut self) -> Promise {
        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_CREATE_DAO_NOT_ALLOWED");
        }

        if self.is_dao_created() {
            env::panic_str("ERR_DAO_ALREADY_CREATED");
        }

        let args = self.get_dao_config(self.metadata.id.clone(),
            vec![env::current_account_id().to_string()],
            vec![env::current_account_id().to_string()],
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

    /**
     * Only if dao it created and if stake it not created
     * Create a new stacking contract
     */
    #[payable]
    pub fn create_stake(&mut self) -> Promise {
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

    /**
     * Only if staking it created and if the dao it not setuped
     * Setup the staking contract
     * Setup a new policy with board and investor accounts
     */
    #[payable]
    pub fn setup_dao(&mut self) -> Promise {
        if !self.is_stake_created() {
            env::panic_str("ERR_STAKE_IS_NOT_CREATED");
        }

        if self.is_dao_setuped() {
            env::panic_str("ERR_DAO_ALREADY_SETUPED");
        }

        let dao_account: AccountId = format!("{}.{}", self.metadata.id, self.metadata.dao_factory)
            .parse()
            .unwrap();
        let stake_account = format!("{}.{}", self.metadata.id, self.metadata.staking_factory);
        let mut promise: Promise = Promise::new(dao_account.clone());

        // Create Staking Proposal
        promise = promise.function_call(
            "add_proposal".to_string(),
            json!({
                "proposal": {
                    "description": "",
                    "kind": {
                        "SetStakingContract": {
                            "staking_id": stake_account
                        }
                    }
                }
            })
            .to_string()
            .into_bytes(),
            BALANCE_PROPOSAL_BOND,
            GAS_CREATE_DAO_PROPOSAL,
        );

        // Approve Staking Proposal
        promise = promise.function_call(
            "act_proposal".to_string(),
            json!({
                "id": 0,
                "action": "VoteApprove"
            })
            .to_string()
            .into_bytes(),
            0,
            GAS_CREATE_DAO_PROPOSAL,
        );

        // Create Policy Proposal
        promise = promise.function_call(
            "add_proposal".to_string(),
            json!({
                "proposal": {
                    "description": "",
                    "kind": {
                        "ChangePolicy": {
                            "policy": self.get_policy(vec![
                                self.metadata.maintainer.to_string()],
                                self.get_deposit_accounts(),
                            )
                        }
                    }
                }
            })
            .to_string()
            .into_bytes(),
            BALANCE_PROPOSAL_BOND,
            GAS_CREATE_DAO_PROPOSAL,
        );

        // Approve Policy Proposal
        promise = promise.function_call(
            "act_proposal".to_string(),
            json!({
                "id": 1,
                "action": "VoteApprove"
            })
            .to_string()
            .into_bytes(),
            0,
            GAS_CREATE_DAO_PROPOSAL,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_create_proposals_callback".to_string(),
            json!({}).to_string().into_bytes(),
            0,
            GAS_CREATE_DAO_PROPOSAL_CB,
        );

        promise.then(callback)
    }
}

impl Escrow {
    fn get_policy(&self, council: Vec<String>, investors: Vec<String>) -> Value {
        json!({
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
        })
    }

    fn get_dao_config(&self, name: String, council: Vec<String>, investors: Vec<String>) -> Vec<u8> {
        json!({
            "policy": self.get_policy(council, investors),
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
