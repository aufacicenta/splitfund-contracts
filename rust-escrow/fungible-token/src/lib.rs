use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LazyOption};
use near_sdk::json_types::{U128};
use near_sdk::serde_json::{json};
use near_sdk::{env, near_bindgen, AccountId, Gas, PanicOnDefault, Promise, PromiseOrValue, PromiseResult};

// Amount of gas used
pub const GAS_FOR_ESCROW_CALL: Gas = Gas(5_000_000_000_000);
pub const GAS_FOR_CLAIM_CALLBACK: Gas = Gas(5_000_000_000_000);

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Ft {
    // Max supply of the token
    max_supply: U128,
    escrow_account_id: AccountId,
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

#[near_bindgen]
impl Ft {
    /// Initializes the contract
    #[init]
    pub fn new(
        max_supply: U128,
        escrow_account_id: AccountId,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        Self {
            max_supply,
            escrow_account_id,
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        }
    }

    pub fn ft_max_supply(&self) -> U128 {
        self.max_supply.into()
    }

    pub fn claim(&mut self, account_id: AccountId) -> Promise {        
        if self.token.accounts.get(&account_id).is_some() {
            env::panic_str("The account has already claimed tokens!");
        }

        // Get balances
        let promise = Promise::new(self.escrow_account_id.clone())
            .function_call(
                "proportion_deposit_of".to_string(),
                json!({"payee": account_id.to_string()})
                    .to_string()
                    .into_bytes(),
                0,
                GAS_FOR_ESCROW_CALL,
            );

        let callback = Promise::new(env::current_account_id()) // the recipient of this ActionReceipt (&self)
            .function_call(
                "on_claim_callback".to_string(), // the function call will be a callback function
                json!({"account_id": account_id.to_string()})
                    .to_string()
                    .into_bytes(),               // method arguments
                0,                               // amount of yoctoNEAR to attach
                GAS_FOR_CLAIM_CALLBACK,          // gas to attach
            );

        promise.then(callback)
    }

    #[private]
    pub fn on_claim_callback(&mut self, account_id: AccountId) {
        match env::promise_result(0) {
            PromiseResult::Successful(result) => {
                let proportion: u128 = near_sdk::serde_json::from_slice(&result).unwrap();

                if proportion == 0 {
                    env::panic_str("You don't have tokens to claim!");
                }
                
                let amount = self.max_supply.0 * proportion / 1000; 

                if let Some(new_total) = self.token.total_supply.checked_add(amount) {
                    if new_total > self.max_supply.0 {
                        env::panic_str("The max supply must not be exceeded!");
                    }
                } else {
                    env::panic_str("Total supply overflow");
                }
                
                self.token.internal_register_account(&account_id);
                self.token.internal_deposit(&account_id, amount);
            },
            _ => env::panic_str("Error"),
        }
    }
}

near_contract_standards::impl_fungible_token_core!(Ft, token);
near_contract_standards::impl_fungible_token_storage!(Ft, token);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Ft {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let metadata = FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "Example Token Name".to_string(),
            symbol: "EXLT".to_string(),
            icon : None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        };
        let contract = Contract::new(accounts(1).into(), TOTAL_SUPPLY.into(), metadata);
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let metadata = FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "Example Token Name".to_string(),
            symbol: "EXLT".to_string(),
            icon : None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        };
        let mut contract = Contract::new(accounts(2).into(), TOTAL_SUPPLY.into(), metadata);
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
