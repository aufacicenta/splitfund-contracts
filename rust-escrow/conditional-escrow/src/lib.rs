use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedMap;
use near_sdk::{env, ext_contract, log, near_bindgen, Gas};
use near_sdk::{AccountId, Balance, Promise, PromiseResult};

/// Amount of gas
pub const GAS_FOR_DELEGATE: Gas = Gas(200_000_000_000_000);
pub const GAS_FOR_DELEGATE_CALLBACK: Gas = Gas(2_000_000_000_000);

// define the methods we'll use on the other contract
#[ext_contract(ext_dao_factory)]
pub trait ExtDaoFactory {
    fn create_dao(&mut self, dao_name: String, deposits: Vec<(AccountId, Balance)>);
}

// define methods we'll use as callbacks on our contract
#[ext_contract(ext_self)]
pub trait MyContract {
    fn on_create_dao_callback(&mut self) -> bool;
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct ConditionalEscrow {
    deposits: UnorderedMap<AccountId, Balance>,
    expires_at: u64,
    total_funds: Balance,
    funding_amount_limit: u128,
    unpaid_funding_amount: u128,
    recipient_account_id: AccountId,
}

impl Default for ConditionalEscrow {
    fn default() -> Self {
        env::panic_str("ConditionalEscrow should be initialized before usage")
    }
}

#[near_bindgen]
impl ConditionalEscrow {
    #[init]
    pub fn new(
        expires_at: u64,
        funding_amount_limit: u128,
        recipient_account_id: AccountId,
    ) -> Self {
        assert!(!env::state_exists(), "The contract is already initialized");
        Self {
            deposits: UnorderedMap::new(b"r".to_vec()),
            total_funds: 0,
            funding_amount_limit,
            unpaid_funding_amount: funding_amount_limit,
            expires_at,
            recipient_account_id,
        }
    }

    pub fn deposits_of(&self, payee: &AccountId) -> Balance {
        match self.deposits.get(payee) {
            Some(deposit) => deposit,
            None => 0,
        }
    }

    pub fn get_deposits(&self) -> Vec<(AccountId, Balance)> {
        self.deposits.to_vec()
    }

    pub fn get_total_funds(&self) -> Balance {
        self.total_funds
    }

    pub fn get_expiration_date(&self) -> u64 {
        self.expires_at
    }

    pub fn get_funding_amount_limit(&self) -> u128 {
        self.funding_amount_limit
    }

    pub fn get_unpaid_funding_amount(&self) -> u128 {
        self.unpaid_funding_amount
    }

    pub fn get_recipient_account_id(&self) -> AccountId {
        self.recipient_account_id.clone()
    }

    pub fn is_deposit_allowed(&self) -> bool {
        !self.has_contract_expired() && !self.is_funding_reached()
    }

    pub fn is_withdrawal_allowed(&self) -> bool {
        self.has_contract_expired() && !self.is_funding_reached()
    }

    #[payable]
    pub fn deposit(&mut self) {
        assert_ne!(
            env::current_account_id(),
            env::signer_account_id(),
            "ERR_OWNER_SHOULD_NOT_DEPOSIT"
        );

        assert!(self.is_deposit_allowed(), "ERR_DEPOSIT_NOT_ALLOWED");
        assert!(
            env::attached_deposit() <= self.get_unpaid_funding_amount(),
            "ERR_DEPOSIT_NOT_ALLOWED"
        );

        let amount = env::attached_deposit();
        let payee = env::signer_account_id();
        let current_balance = self.deposits_of(&payee);
        let new_balance = &(current_balance.wrapping_add(amount));

        self.deposits.insert(&payee, new_balance);
        self.total_funds = self.total_funds.wrapping_add(amount);
        self.unpaid_funding_amount = self.unpaid_funding_amount.wrapping_sub(amount);

        log!(
            "{} deposited {} NEAR tokens. New balance {} — Total funds: {} — Unpaid funds: {}",
            &payee,
            amount,
            new_balance,
            self.total_funds,
            self.unpaid_funding_amount
        );
        // @TODO emit deposit event
    }

    #[payable]
    pub fn withdraw(&mut self) {
        assert!(self.is_withdrawal_allowed(), "ERR_WITHDRAWAL_NOT_ALLOWED");

        let payee = env::signer_account_id();
        let payment = self.deposits_of(&payee);

        Promise::new(payee.clone()).transfer(payment);
        self.deposits.insert(&payee, &0);
        self.total_funds = self.total_funds.wrapping_sub(payment);
        self.unpaid_funding_amount = self.unpaid_funding_amount.wrapping_add(payment);

        log!(
            "{} withdrawn {} NEAR tokens. New balance {} — Total funds: {} — Unpaid funds: {}",
            &payee,
            payment,
            self.deposits_of(&payee),
            self.total_funds,
            self.unpaid_funding_amount
        );
        // @TODO emit withdraw event
    }

    #[payable]
    pub fn delegate_funds(&mut self, dao_name: String) -> Promise {
        assert!(
            !(self.is_deposit_allowed() || self.is_withdrawal_allowed()),
            "ERR_DELEGATE_NOT_ALLOWED"
        );

        let recipient_contract_id = self.get_recipient_account_id();
        let total_funds = self.get_total_funds();

        // @TODO charge a fee here (1.5% initially?) when a property is sold by our contract

        ext_dao_factory::create_dao(
            dao_name.clone(),
            self.deposits.to_vec(),
            recipient_contract_id.clone(),
            total_funds,
            GAS_FOR_DELEGATE,
        )
        .then(ext_self::on_create_dao_callback(
            env::current_account_id(),
            0,
            GAS_FOR_DELEGATE_CALLBACK,
        ))

        // @TODO emit delegate_funds event
    }

    #[private]
    pub fn on_create_dao_callback(&mut self) -> bool {
        assert_eq!(env::promise_results_count(), 1, "ERR_CALLBACK_METHOD");

        match env::promise_result(0) {
            PromiseResult::Successful(_result) => true,
            _ => panic!("ERR_CREATE_DAO_UNSUCCESSFUL"),
        }
    }

    fn has_contract_expired(&self) -> bool {
        self.expires_at < env::block_timestamp().try_into().unwrap()
    }

    fn is_funding_reached(&self) -> bool {
        self.get_total_funds() >= self.get_funding_amount_limit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use near_sdk::test_utils::test_env::{alice, bob, carol};
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, PromiseResult};

    const ATTACHED_DEPOSIT: Balance = 8_540_000_000_000_000_000_000;
    const MIN_FUNDING_AMOUNT: u128 = 1_000_000_000_000_000_000_000_000;

    fn setup_context() -> VMContextBuilder {
        let mut context = VMContextBuilder::new();
        let now = Utc::now().timestamp_subsec_nanos();
        testing_env!(context
            .predecessor_account_id(alice())
            .block_timestamp(now.try_into().unwrap())
            .build());

        context
    }

    fn setup_contract(expires_at: u64, funding_amount_limit: u128) -> ConditionalEscrow {
        let contract = ConditionalEscrow::new(expires_at, funding_amount_limit, accounts(3));

        contract
    }

    fn add_expires_at_nanos(offset: u32) -> u64 {
        let now = Utc::now().timestamp_subsec_nanos();
        (now + offset).into()
    }

    fn substract_expires_at_nanos(offset: u32) -> u64 {
        let now = Utc::now().timestamp_subsec_nanos();
        (now - offset).into()
    }

    #[test]
    fn test_get_deposits_of() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            0,
            contract.deposits_of(&alice()),
            "Account deposits should be 0"
        );
    }

    #[test]
    fn test_get_deposits() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            contract.get_deposits(),
            vec![(bob(), ATTACHED_DEPOSIT)],
            "Gets all deposits as vec"
        );
    }

    #[test]
    fn test_get_recipient_account_id() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(
            accounts(3),
            contract.get_recipient_account_id(),
            "Recipient account id should be 'danny.near'"
        );
    }

    #[test]
    fn test_get_0_total_funds() {
        let expires_at = add_expires_at_nanos(100);

        let contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");
    }

    #[test]
    fn test_get_correct_unpaid_funding_amount() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            MIN_FUNDING_AMOUNT - ATTACHED_DEPOSIT,
            contract.get_unpaid_funding_amount(),
            "Unpaid funding amount is wrong"
        );
    }

    #[test]
    fn test_get_total_funds_after_deposits() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            ATTACHED_DEPOSIT * 2,
            contract.get_total_funds(),
            "Total funds should be ATTACHED_DEPOSITx2"
        );
    }

    #[test]
    fn test_is_withdrawal_allowed() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT * 2);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT - 1_000)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT - 1_000)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(bob())
            .block_timestamp((expires_at + 100).try_into().unwrap())
            .build());

        contract.withdraw();

        testing_env!(context.signer_account_id(carol()).build());

        contract.withdraw();

        assert_eq!(
            true,
            contract.is_withdrawal_allowed(),
            "Withdrawal should be allowed"
        );

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");
    }

    #[test]
    #[should_panic(expected = "ERR_WITHDRAWAL_NOT_ALLOWED")]
    fn test_is_withdrawal_not_allowed() {
        setup_context();
        let expires_at = add_expires_at_nanos(1_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.withdraw();

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT_NOT_ALLOWED")]
    fn test_is_deposit_not_allowed_by_expiration_date() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        let expires_at = substract_expires_at_nanos(1_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_DEPOSIT_NOT_ALLOWED")]
    fn test_is_deposit_not_allowed_by_total_funds_reached() {
        let mut context = setup_context();

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT)
            .build());

        let expires_at = add_expires_at_nanos(1_000_000);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );
    }

    #[test]
    #[should_panic(expected = "ERR_OWNER_SHOULD_NOT_DEPOSIT")]
    fn test_owner_deposit() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(alice())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();
    }

    #[test]
    #[should_panic(expected = "ERR_DELEGATE_NOT_ALLOWED")]
    fn test_should_not_delegate_funds_if_active() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(ATTACHED_DEPOSIT)
            .build());

        contract.deposit();

        assert_eq!(
            true,
            contract.is_deposit_allowed(),
            "Deposit should be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_DELEGATE_NOT_ALLOWED")]
    fn test_should_not_delegate_funds_if_expired() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT - 1_000)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            true,
            contract.is_withdrawal_allowed(),
            "Withdrawal should be allowed"
        );

        contract.delegate_funds("dao1".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_DELEGATE_NOT_ALLOWED")]
    fn test_should_not_delegate_funds_if_already_delegated() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("true".to_string().into_bytes())],
        );

        contract.on_create_dao_callback();

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");

        contract.delegate_funds("dao1".to_string());
    }

    #[test]
    #[should_panic(expected = "ERR_CREATE_DAO_UNSUCCESSFUL")]
    fn test_should_not_delegate_funds_if_create_dao_fails() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("false".to_string().into_bytes())],
        );

        contract.on_create_dao_callback();

        assert_eq!(
            MIN_FUNDING_AMOUNT,
            contract.get_total_funds(),
            "Total funds should be MIN_FUNDING_AMOUNT"
        );
    }

    #[test]
    fn test_delegate_funds() {
        let mut context = setup_context();

        let expires_at = add_expires_at_nanos(100);

        let mut contract = setup_contract(expires_at, MIN_FUNDING_AMOUNT);

        testing_env!(context
            .signer_account_id(bob())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .signer_account_id(carol())
            .attached_deposit(MIN_FUNDING_AMOUNT / 2)
            .build());

        contract.deposit();

        testing_env!(context
            .block_timestamp((expires_at + 200).try_into().unwrap())
            .build());

        assert_eq!(
            false,
            contract.is_deposit_allowed(),
            "Deposit should not be allowed"
        );

        assert_eq!(
            false,
            contract.is_withdrawal_allowed(),
            "Withdrawal should not be allowed"
        );

        contract.delegate_funds("dao1".to_string());

        testing_env!(
            context.build(),
            near_sdk::VMConfig::test(),
            near_sdk::RuntimeFeesConfig::test(),
            Default::default(),
            vec![PromiseResult::Successful("true".to_string().into_bytes())],
        );

        contract.on_create_dao_callback();

        assert_eq!(0, contract.get_total_funds(), "Total funds should be 0");

        assert_eq!(
            MIN_FUNDING_AMOUNT / 2,
            contract.deposits_of(&bob()),
            "Account deposits should be MIN_FUNDING_AMOUNT"
        );

        assert_eq!(
            MIN_FUNDING_AMOUNT / 2,
            contract.deposits_of(&carol()),
            "Account deposits should be MIN_FUNDING_AMOUNT"
        );
    }
}
