use near_sdk::{
    collections::{LazyOption, UnorderedMap},
    env, json_types::U128, log, near_bindgen, serde_json::json,
    AccountId, Balance, Promise, PromiseOrValue,
};

use near_contract_standards::fungible_token::{
    FungibleToken,
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider},
};

//use crate::consts::*;
use crate::storage::*;

impl Default for Escrow {
    fn default() -> Self {
        env::panic_str("Escrow Contract should be initialized before usage")
    }
}

#[near_bindgen]
impl Escrow {
    #[init]
    pub fn new(metadata: Metadata) -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

        Self {
            deposits: UnorderedMap::new(b"d".to_vec()),
            ft: FungibleToken::new(b"t".to_vec()),
            ft_metadata: LazyOption::new(b"m".to_vec(), None),
            metadata,
        }
    }

    /*
    /**
     * Called by anyone only once, this creates the NEP141 own token to be transfered in exchange of the stable NEP141 deposits
     */
    pub fn publish(&mut self, sender_id: AccountId, amount: Balance) {}

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

        if amount > self.get_unpaid_funding_amount() {
            env::panic_str("ERR_DEPOSIT_NOT_ALLOWED");
        }

        // @TODO charge fee?

        // @TODO create promise to transfer self NEP141 in exchange of the amount as a receipt, then update storage values

        let current_balance = self.deposits_of(&sender_id);
        let new_balance = &(current_balance.wrapping_add(amount));
        self.deposits.insert(&sender_id, new_balance);
        self.metadata.unpaid_funding_amount =
            self.metadata.unpaid_funding_amount.wrapping_sub(amount);

        // @TODO log
    }

    /**
     * Transfer funds from contract NEP141 balance to sender_id
     */
    pub fn withdraw(&mut self) {
        if !self.is_withdrawal_allowed() {
            env::panic_str("ERR_WITHDRAWAL_NOT_ALLOWED");
        }

        let payee = env::signer_account_id();
        let payment = self.deposits_of(&payee);

        // @TODO perform ft_transfer from contract to sender, then update storage on promise callback

        self.deposits.insert(&payee, &0);
        self.metadata.unpaid_funding_amount =
            self.metadata.unpaid_funding_amount.wrapping_add(payment);

        // @TODO log
    }

    /**
     * Only if total funds are reached, allow to call this function
     * Transfer total NEP141 funds to a new DAO
     * Make the depositors members of the DAO
     */
    pub fn delegate_funds(&mut self, dao_name: String) -> Promise {
        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_DELEGATE_NOT_ALLOWED");
        }

        // env::panic_str("ERR_TOTAL_FUNDS_OVERFLOW");

        // @TODO charge a fee here (1.5% initially?) when a property is sold by our contract

        let dao_promise = Promise::new(self.metadata.dao_factory_account_id.clone()).function_call(
            "create_dao".to_string(),
            json!({"dao_name": dao_name.clone(), "deposits": self.get_deposit_accounts() })
                .to_string()
                .into_bytes(),
            // @TODO check deposit value to create_dao
            FT_ATTACHED_DEPOSIT,
            GAS_FOR_CREATE_DAO,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_delegate_callback".to_string(),
            json!({"dao_name": dao_name.clone()})
                .to_string()
                .into_bytes(),
            0,
            GAS_FOR_CALLBACK,
        );

        dao_promise.then(callback)
    }
    */
}

near_contract_standards::impl_fungible_token_core!(Escrow, ft);
near_contract_standards::impl_fungible_token_storage!(Escrow, ft);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Escrow {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.ft_metadata.get().unwrap()
    }
}
