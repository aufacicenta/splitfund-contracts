#[cfg(test)]
mod tests {
    use chrono::Utc;
    use near_contract_standards::fungible_token::core::FungibleTokenCore;
    use near_contract_standards::fungible_token::metadata::FungibleTokenMetadata;
    use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
    use near_contract_standards::storage_management::StorageManagement;
    use near_sdk::PromiseResult;
    use near_sdk::{
        json_types::U128,
        test_utils::{
            accounts,
            test_env::{alice, bob},
            VMContextBuilder,
        },
        testing_env, AccountId, Balance,
    };
    //use near_sdk::PromiseOrValue::Value;

    use crate::storage::*;

    const ATTACHED_DEPOSIT: Balance = 1_000_000_000_000_000_000_000_000; // 1 Near
    const MIN_FUNDING_AMOUNT: Balance = 1_000_000;

    fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    fn nep_141_account_id() -> AccountId {
        AccountId::new_unchecked("nep141.near".to_string())
    }

    fn maintainer_account_id() -> AccountId {
        AccountId::new_unchecked("maintainer.near".to_string())
    }

    fn fees_account_id() -> AccountId {
        AccountId::new_unchecked("fees.near".to_string())
    }

    fn new_metadata(
        expires_at: Timestamp,
        funding_amount_limit: u128,
        nep_141: Option<AccountId>,
        maintainer: Option<AccountId>,
    ) -> Metadata {
        let nep_141 = nep_141.unwrap_or(nep_141_account_id());
        let maintainer = maintainer.unwrap_or(maintainer_account_id());

        Metadata {
            expires_at,
            funding_amount_limit,
            unpaid_amount: 0,
            nep_141,
            maintainer_account_id: maintainer,
            metadata_url: "".to_string(),
        }
    }

    fn new_fees(percentage: f32, account_id: Option<AccountId>) -> Fees {
        let account_id = account_id.unwrap_or(fees_account_id());

        Fees {
            percentage,
            amount: 0,
            account_id,
            claimed: false,
        }
    }

    fn new_ft_metadata(name: String, decimals: u8) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: name.clone(),
            symbol: name.clone(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals,
        }
    }

    fn setup_contract(expires_at: u64, funding_amount_limit: u128) -> Escrow {
        let metadata = new_metadata(expires_at, funding_amount_limit, None, None);
        let fees = new_fees(0.03, None);
        let ft_metadata = new_ft_metadata("sa1".to_string(), 4);

        let contract = Escrow::new(metadata, fees, ft_metadata, None);
        contract
    }

    fn register_account(contract: &mut Escrow, account: AccountId) {
        let mut context = get_context(account.clone());

        testing_env!(context.attached_deposit(ATTACHED_DEPOSIT).build());

        contract.ft.storage_deposit(Some(account), None);
    }

    fn add_expires_at_nanos(offset: u32) -> u64 {
        let now = Utc::now().timestamp_subsec_nanos();
        (now + offset).into()
    }

    #[test]
    #[should_panic(expected = "Escrow Contract should be initialized before usage")]
    fn default_state_err() {
        Escrow::default();
    }

    //################
    // Test On Deposit

    #[test]
    #[should_panic(expected = "ERR_WRONG_NEP141")]
    fn deposit_wrong_nep141_err() {
        let context = get_context(accounts(1));
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.ft_on_transfer(bob(), U128(100_000), "".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_ZERO_AMOUNT")]
    fn deposit_zero_amount_err() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.ft_on_transfer(bob(), U128(0), "".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT_NOT_ALLOWED")]
    fn deposit_not_allowed_by_expiration_date() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        register_account(&mut contract, bob());

        let mut context = get_context(nep_141_account_id());
        testing_env!(context.block_timestamp(expires_at + 1000).build());

        contract.ft_on_transfer(bob(), U128(100_000), "".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_AMOUNT_GT_UNPAID_AMOUNT")]
    fn deposit_amount_gt_unpaid_amount() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        // Bob Deposit
        let bob_investment = 900_000;

        register_account(&mut contract, bob());

        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        contract.ft_on_transfer(bob(), U128(bob_investment), "".to_string());

        // Bob Deposit x2
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        contract.ft_on_transfer(bob(), U128(bob_investment), "".to_string());
    }

    #[test]
    fn deposit_success() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        // Bob Deposit
        let bob_investment = 100_000;

        register_account(&mut contract, bob());

        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        contract.ft_on_transfer(bob(), U128(bob_investment), "".to_string());

        // Alice Deposit
        let alice_investment = 45_000;

        register_account(&mut contract, alice());

        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        contract.ft_on_transfer(alice(), U128(alice_investment), "".to_string());

        // Check balances

        let amount_bob = contract.ft.ft_balance_of(bob());
        let amount_alice = contract.ft.ft_balance_of(alice());
        let fees = contract.get_fees();

        assert_eq!(
            bob_investment + alice_investment,
            amount_bob.0 + amount_alice.0 + fees.amount,
            "Investment should be equal to amount + fees"
        );
    }

    //#################
    // Test On Withdraw

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn withdraw_1yocto_error() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.withdraw();
    }

    #[test]
    #[should_panic(expected = "ERR_WITHDRAWAL_NOT_ALLOWED")]
    fn withdraw_not_allowed_error() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        let mut context = get_context(bob());

        testing_env!(context.attached_deposit(1).build());

        contract.withdraw();
    }

    #[test]
    fn withdraw_success() {
        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        let expires_at = add_expires_at_nanos(100);
        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        // Bob Deposit
        let bob_investment = 100_000;

        register_account(&mut contract, bob());

        let context = get_context(nep_141_account_id());
        testing_env!(context.build());

        contract.ft_on_transfer(bob(), U128(bob_investment), "".to_string());

        let amount_bob = contract.ft.ft_balance_of(bob());

        assert_eq!((bob_investment as f32 * 0.97) as u128, amount_bob.0);

        // Bob Withdraw
        register_account(&mut contract, bob());

        let mut context = get_context(bob());

        testing_env!(context
            .block_timestamp(expires_at + 1000)
            .attached_deposit(1)
            .build());

        contract.withdraw();

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful(vec![])],
        );

        contract.on_withdraw_callback(bob(), amount_bob);

        assert_eq!(0, contract.ft.ft_balance_of(bob()).0);
    }
}
