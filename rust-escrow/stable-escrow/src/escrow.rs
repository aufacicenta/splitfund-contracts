use near_sdk::{
    assert_one_yocto,
    collections::{LazyOption, UnorderedSet},
    env, ext_contract,
    json_types::U128,
    log, near_bindgen,
    serde_json::json,
    AccountId, Balance, Promise, PromiseOrValue,
};

use near_contract_standards::fungible_token::{
    core::ext_ft_core,
    metadata::{FungibleTokenMetadata, FungibleTokenMetadataProvider},
    FungibleToken,
};

//use crate::callbacks::*;
use crate::consts::*;
use crate::storage::*;

// Interface of this contract, for callbacks
#[ext_contract(ext_self)]
trait Callbacks {
  fn on_withdraw_callback(&mut self, receiver_id: AccountId, amount: U128) -> Balance;
  fn on_claim_fees_callback(&mut self, amount: U128) -> bool;
}

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
        fungible_token_metadata: FungibleTokenMetadata,
        storage_deposit_amount: Option<Balance>,
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

        // Fungible Token Setup
        let mut ft = FungibleToken::new(b"t".to_vec());
        ft.total_supply = metadata.funding_amount_limit;
        ft.internal_register_account(&metadata.maintainer_account_id);

        // Deposits Setup
        let mut deposits = UnorderedSet::new(b"d".to_vec());
        deposits.insert(&metadata.maintainer_account_id);

        // Escrow Storage Deposit
        let storage_deposit_amount = storage_deposit_amount.
            unwrap_or(BALANCE_ON_STORAGE_DEPOSIT);

        Promise::new(metadata.nep_141.clone()).function_call(
            "storage_deposit".to_string(),
            json!({})
            .to_string()
            .into_bytes(),
            storage_deposit_amount,
            GAS_ON_TRANSFER,
        );

        let mut this = Self {
            deposits,
            ft,
            ft_metadata: LazyOption::new(b"m".to_vec(), Some(&ft_metadata)),
            metadata: Metadata {
                unpaid_amount: metadata.funding_amount_limit,
                ..metadata
            },
            fees: Fees {
                amount: 0,
                claimed: false,
                ..fees
            },
            account_storage_usage: 0,
        };

        this.measure_account_storage_usage();
        this
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
        self.fees.amount = self
            .get_fees()
            .amount
            .checked_add(fee_amount)
            .unwrap_or_else(|| env::panic_str("ERR_FEES_AMOUNT_OVERFLOW"));
        let amount_minus_fee = amount
            .checked_sub(fee_amount)
            .unwrap_or_else(|| env::panic_str("ERR_AMOUNT_MINUS_FEE_OVERFLOW"));

        // Register transfer
        self.ft.internal_deposit(&sender_id, amount_minus_fee);
        self.deposits.insert(&sender_id);
        self.metadata.unpaid_amount = self
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
        assert_one_yocto();

        if !self.is_withdrawal_allowed() {
            env::panic_str("ERR_WITHDRAWAL_NOT_ALLOWED");
        }

        let receiver_id = env::signer_account_id();
        let amount = U128(self.ft.internal_unwrap_balance_of(&receiver_id));
        
        // NEP141 Transfer
        let promise = ext_ft_core::ext(self.get_metadata().nep_141.clone())
            .with_attached_deposit(1)
            .with_static_gas(GAS_ON_TRANSFER)
            .ft_transfer(receiver_id.clone(), amount, None);

        let callback = ext_self::ext(env::current_account_id())
            .with_static_gas(GAS_ON_TRANSFER_CB)
            .with_attached_deposit(0)
            .on_withdraw_callback(receiver_id.clone(), amount);

        promise.then(callback)
    }

    #[payable]
    pub fn claim_fees(&mut self) -> Promise {
        assert_one_yocto();

        if self.is_deposit_allowed() {
            env::panic_str("ERR_CLAIM_FEES_NOT_ALLOWED");
        }

        if self.get_fees().claimed {
            env::panic_str("ERR_FEES_ALREADY_CLAIMED");
        }

        let fees_amount = U128(self.get_fees().amount);
        let receiver_id = self.get_fees().account_id.clone();

        // NEP141 Transfer
        let promise = ext_ft_core::ext(self.get_metadata().nep_141.clone())
            .with_attached_deposit(1)
            .with_static_gas(GAS_ON_TRANSFER)
            .ft_transfer(receiver_id.clone(), fees_amount, None);

        let callback = ext_self::ext(env::current_account_id())
            .with_static_gas(GAS_ON_TRANSFER_CB)
            .with_attached_deposit(0)
            .on_claim_fees_callback(fees_amount);

        promise.then(callback)
    }

    /**
     * Only if total funds are reached, allow to call this function
     * Transfer total NEP141 funds to a new DAO
     * Make the depositors members of the DAO
     */
    #[payable]
    pub fn delegate_funds(&mut self, amount: Option<U128>) -> Promise {
        assert_one_yocto();

        if self.is_deposit_allowed() || self.is_withdrawal_allowed() {
            env::panic_str("ERR_DELEGATE_NOT_ALLOWED");
        }

        let fees_amount = self.get_fees().amount;
        let receiver_id = self.get_metadata().maintainer_account_id.clone();
        
        // If amount is None then use the funding_amount_limit
        let amount = amount.unwrap_or(U128(self.get_metadata().funding_amount_limit));

        // Amount to delegate minus fees collected
        let amount_minus_fee = amount
            .0
            .checked_sub(fees_amount)
            .unwrap_or_else(|| env::panic_str("ERR_AMOUNT_MINUS_FEE_OVERFLOW"));

        log!(
            "[on_delegate_funds]: receiver_id: {}, amount: {}",
            receiver_id,
            amount_minus_fee
        );

        // NEP141 Transfer
        ext_ft_core::ext(self.get_metadata().nep_141.clone())
            .with_attached_deposit(1)
            .with_static_gas(GAS_ON_TRANSFER)
            .ft_transfer(receiver_id.clone(), U128(amount_minus_fee), None)
    }
}

impl Escrow {
    fn measure_account_storage_usage(&mut self) {
        let initial_storage_usage = env::storage_usage();
        let tmp_account_id = AccountId::new_unchecked("a".repeat(64));
        self.deposits.insert(&tmp_account_id);
        self.ft.accounts.insert(&tmp_account_id, &0u128);
        self.account_storage_usage = env::storage_usage() - initial_storage_usage;
        self.ft.accounts.remove(&tmp_account_id);
        self.deposits.remove(&tmp_account_id);
    }
}

near_contract_standards::impl_fungible_token_core!(Escrow, ft);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Escrow {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.ft_metadata.get().unwrap()
    }
}
