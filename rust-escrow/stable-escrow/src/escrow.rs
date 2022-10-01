use near_sdk::{
    AccountId, Balance, Promise, PromiseOrValue,
    collections::{LazyOption, UnorderedSet},
    env, json_types::U128, log, near_bindgen, serde_json::json,
};

use near_contract_standards::fungible_token::{
    FungibleToken,
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider},
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
        expires_at: u64,
        funding_amount_limit: U128,
        nep_141_account_id: AccountId,
        dao_factory_account_id: AccountId,
        metadata_url: String,
    ) -> Self {
        if env::state_exists() {
            env::panic_str("ERR_ALREADY_INITIALIZED");
        }

        //@TODO Define ft_metadata (token name, token symbol, decimals)

        let mut token = FungibleToken::new(b"t".to_vec());
        token.total_supply = funding_amount_limit.0;

        Self {
            deposits: UnorderedSet::new(b"d".to_vec()),
            ft: token,
            ft_metadata: LazyOption::new(b"m".to_vec(), None),
            metadata: Metadata {
                expires_at,
                funding_amount_limit: funding_amount_limit.0,
                unpaid_amount: funding_amount_limit.0,
                nep_141_account_id,
                dao_factory_account_id,
                metadata_url
            }
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
        self.metadata.unpaid_amount = self.
            metadata.unpaid_amount.
            checked_sub(amount).
            unwrap_or_else(|| env::panic_str("ERR_UNPAID_AMOUNT_OVERFLOW"));

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
        let promise = Promise::new(self.metadata.nep_141_account_id.clone()).function_call(
            "ft_transfer".to_string(),
            json!({
                "amount": balance.to_string(),
                "receiver_id": payee.to_string(),
            }).to_string().into_bytes(),
            1, // 1 yoctoNEAR
            GAS_FT_TRANSFER,
        );

        let callback = Promise::new(env::current_account_id()).function_call(
            "on_withdraw_callback".to_string(),
            json!({
                "payee": payee.to_string(),
                "balance": balance.to_string(),
            }).to_string().into_bytes(),
            0,
            GAS_FT_WITHDRAW_CALLBACK,
        );

        promise.then(callback)
    }

    /*
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
