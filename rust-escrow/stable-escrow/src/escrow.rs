use near_sdk::{
    collections::{LazyOption, UnorderedSet},
    env,
    json_types::{Base64VecU8, U128},
    log, near_bindgen,
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
        metadata: Metadata,
        fees: Fees,
        dao: DAO,
        staking: Staking,
        fungible_token_metadata: FungibleTokenMetadata,
    ) -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

        let ft_metadata = FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            ..fungible_token_metadata
        };

        let mut ft = FungibleToken::new(b"t".to_vec());
        ft.total_supply = metadata.funding_amount_limit;
        ft.internal_register_account(&metadata.maintainer_account_id);

        let mut deposits = UnorderedSet::new(b"d".to_vec());
        deposits.insert(&metadata.maintainer_account_id);

        Self {
            deposits,
            ft,
            ft_metadata: LazyOption::new(b"m".to_vec(), Some(&ft_metadata)),
            metadata: Metadata {
                unpaid_amount: metadata.funding_amount_limit,
                ..metadata
            },
            dao: DAO {
                created_at: None,
                setup_at: None,
                factory_account_id: dao.factory_account_id,
            },
            fees: Fees { balance: 0, ..fees },
            staking: Staking {
                created_at: None,
                ..staking
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

        if amount > self.get_metadata().unpaid_amount {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        // Fee Calculations
        let fee_amount = (amount as f32 * self.get_fees().percentage) as Balance;
        self.fees.balance = self
            .get_fees()
            .balance
            .checked_add(fee_amount)
            .unwrap_or_else(|| env::panic_str("ERR_FEE_BALANCE_OVERFLOW"));
        let amount_minus_fee = amount
            .checked_sub(fee_amount)
            .unwrap_or_else(|| env::panic_str("ERR_AMOUNT_OVERFLOW"));

        // Register transfer
        self.ft.internal_deposit(&sender_id, amount_minus_fee);
        self.deposits.insert(&sender_id);
        self.get_metadata().unpaid_amount = self
            .metadata
            .unpaid_amount
            .checked_sub(amount)
            .unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

        log!(
            "[deposit]: sender_id: {}, amount_minus_fee: {}, fee: {}",
            sender_id,
            amount_minus_fee,
            fee_amount
        );
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

        let promise = Promise::new(self.get_metadata().nep_141.clone()).function_call(
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

        if self.get_fees().balance == 0 {
            env::panic_str("ERR_FEES_ALREADY_CLAIMED");
        }

        let mut amount = self.get_fees().balance;

        if !self.is_deposit_allowed() && !self.is_withdrawal_allowed() {
            // If the escrow is successful
            // Half of the fees are collected and half invested in the asset
            amount = (self.get_fees().balance as f32 * 0.5) as Balance;
        }

        let receiver_id = self.get_metadata().maintainer_account_id.clone();

        let promise = Promise::new(self.get_metadata().nep_141.clone()).function_call(
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

        amount = self
            .get_fees()
            .balance
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

        let name = self.ft_metadata().name;

        let args = self.get_dao_config(
            name.clone(),
            vec![env::current_account_id().to_string()],
            vec![env::current_account_id().to_string()],
        );

        let promise = Promise::new(self.get_dao().factory_account_id).function_call(
            "create".to_string(),
            json!({ "name": name.clone(), "args": Base64VecU8(args) })
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

        let name = self.ft_metadata().name;

        let promise = Promise::new(self.get_staking().factory_account_id).function_call(
            "create_stake".to_string(),
            json!({
                "name": name,
                "dao_account_id": format!("{}.{}", name, self.get_dao().factory_account_id),
                "token_account_id": self.get_metadata().nep_141,
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

        if self.is_dao_setup() {
            env::panic_str("ERR_DAO_ALREADY_SETUPED");
        }

        let name = self.ft_metadata().name;

        let dao_account: AccountId = format!(
            "{}.{}",
            name,
            self.get_dao().factory_account_id
        )
        .parse()
        .unwrap();
        let stake_account = format!(
            "{}.{}",
            name,
            self.get_staking().factory_account_id
        );
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
                                self.get_metadata().maintainer_account_id.to_string()],
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

    fn get_dao_config(
        &self,
        name: String,
        council: Vec<String>,
        investors: Vec<String>,
    ) -> Vec<u8> {
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
