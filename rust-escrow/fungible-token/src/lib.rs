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

    pub fn ft_escrow_account_id(&self) -> AccountId {
        self.escrow_account_id.clone()
    }

    pub fn claim(&mut self, account_id: AccountId) -> Promise {        
        if self.token.accounts.get(&account_id).is_some() {
            env::panic_str("The account has already claimed tokens!");
        }

        // Get balances
        let promise = Promise::new(self.escrow_account_id.clone())
            .function_call(
                "get_shares_of".to_string(),
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
            _ => env::panic_str("Error calling escrow contract"),
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

    const MAX_SUPPLY: Balance = 1_000_000_000_000_000;

    fn get_metadata() -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "Example Token Name".to_string(),
            symbol: "EXLT".to_string(),
            icon : None,
            reference: None,
            reference_hash: None,
            decimals: 8,
        }
    }

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
        let contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());
        testing_env!(context.is_view(true).build());

        assert_eq!(contract.ft_max_supply().0, MAX_SUPPLY);
        assert_eq!(contract.ft_total_supply().0, 0);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, 0);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Ft::default();
    }

    #[test]
    fn test_ft_max_supply() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());

        assert_eq!(contract.ft_max_supply().0, MAX_SUPPLY);
    }

    #[test]
    fn test_ft_escrow_account_id() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());

        assert_eq!(contract.ft_escrow_account_id(), accounts(1));
    }

    #[test]
    fn test_claim() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());

        // Account 2 Claim
        contract.claim(accounts(2).into());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("100".to_string().into_bytes())],
        );

        contract.on_claim_callback(accounts(2).into());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, 100000000000000);

        // Account 3 Claim
        contract.claim(accounts(3).into());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("200".to_string().into_bytes())],
        );

        contract.on_claim_callback(accounts(3).into());
        assert_eq!(contract.ft_balance_of(accounts(3)).0, 200000000000000);

        assert_eq!(contract.ft_max_supply().0, MAX_SUPPLY);
        assert_eq!(contract.ft_total_supply().0, 300000000000000);
    }

    #[test]
    #[should_panic(expected = "The max supply must not be exceeded!")]
    fn test_claim_exceed_max_supply() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());

        // Account 2 Claim
        contract.claim(accounts(2).into());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("1100".to_string().into_bytes())],
        );

        contract.on_claim_callback(accounts(2).into());
    }

    #[test]
    #[should_panic(expected = "You don't have tokens to claim!")]
    fn test_claim_not_allowed() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());

        // Account 2 Claim
        contract.claim(accounts(2).into());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("0".to_string().into_bytes())],
        );

        contract.on_claim_callback(accounts(2).into());
    }

    #[test]
    #[should_panic(expected = "The account has already claimed tokens!")]
    fn test_claim_twice() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let mut contract = Ft::new(MAX_SUPPLY.into(), accounts(1).into(), get_metadata());

        // Account 2 Claim
        contract.claim(accounts(2).into());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("100".to_string().into_bytes())],
        );

        contract.on_claim_callback(accounts(2).into());

        // Account 2 Claim Again
        contract.claim(accounts(2).into());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("100".to_string().into_bytes())],
        );

        contract.on_claim_callback(accounts(2).into());
    }
}
