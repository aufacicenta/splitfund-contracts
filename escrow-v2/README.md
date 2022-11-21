# Escrow NEP141

## Deploy NEP141

```bash
npm install -g near-cli
rustup target add wasm32-unknown-unknown
near dev-deploy --wasmFile res/fungible_token.wasm
source neardev/dev-account.env
near call $CONTRACT_NAME new '{"owner_id": "'$CONTRACT_NAME'", "total_supply": "1000000000000000", "metadata": { "spec": "ft-1.0.0", "name": "Example Token Name", "symbol": "EXLT", "decimals": 8 }}' --accountId $CONTRACT_NAME
```

## Test NEP141

```bash
ID=$CONTRACT_NAME
near create-account bob.$ID --masterAccount $ID --initialBalance 1
near call $ID storage_deposit '' --accountId bob.$ID --amount 0.00125
near call $ID ft_transfer '{"receiver_id": "'bob.$ID'", "amount": "100000000"}' --accountId $ID --amount 0.000000000000000000000001
near view $ID ft_balance_of '{"account_id": "'bob.$ID'"}'
```

## Deploy Escrow

```bash
near delete es1.$ID $ID
near create-account es1.$ID --masterAccount $ID --initialBalance 5
near deploy --wasmFile res/escrow.wasm --accountId es1.$ID
# Vence en diciembre, funding 10000
near call es1.$ID new '{"metadata": {"expires_at": 1670215945000000000, "funding_amount_limit": 10000, "unpaid_amount": 0,  "nep_141": "'$ID'", "maintainer_account_id": "'$ID'", "metadata_url": ""}, "fees": {"percentage": 0.02, "amount": 0, "account_id": "'$ID'", "claimed": false}, "fungible_token_metadata": {"spec": "ft-1.0.0", "name": "sa18", "symbol": "sa18", "decimals": 2}}' --accountId $ID

near view es1.$ID ft_balance_of '{"account_id": "'bob.$ID'"}'
near view es1.$ID ft_total_supply
near view es1.$ID ft_metadata
```

## Deposit

```bash
# Paying for storage
near view es1.$ID storage_balance_bounds
near call es1.$ID storage_deposit '' --accountId bob.$ID --amount 0.00361

near call $ID ft_transfer_call '{"receiver_id": "'es1.$ID'", "amount": "10000", "msg": ""}' --accountId bob.$ID --amount 0.000000000000000000000001 --gas 50000000000000

near view es1.$ID ft_balance_of '{"account_id": "'bob.$ID'"}'
near view $ID ft_balance_of '{"account_id": "'es1.$ID'"}'
near view es1.$ID get_deposit_accounts
```

## Withdraw

```bash
near view es1.$ID is_deposit_allowed
near view es1.$ID is_withdrawal_allowed
near call es1.$ID withdraw --accountId bob.$ID --amount 0.000000000000000000000001
near view es1.$ID ft_balance_of '{"account_id": "'bob.$ID'"}'
```

## Claim Fees

```bash
near call es1.$ID claim_fees --accountId bob.$ID --amount 0.000000000000000000000001
near view es1.$ID get_fees

near view es1.$ID ft_balance_of '{"account_id": "'$ID'"}'
near view es1.$ID ft_balance_of '{"account_id": "'es1.$ID'"}'
near view es1.$ID ft_balance_of '{"account_id": "'bob.$ID'"}'
```

## Delegate Funds

### Delegate all funds

```bash
near call es1.$ID delegate_funds --accountId bob.$ID --amount 0.000000000000000000000001
near view $ID ft_balance_of '{"account_id": "'es1.$ID'"}'
```

### Delegate with amount

```bash
near call es1.$ID delegate_funds '{"amount": "500"}' --accountId bob.$ID --amount 0.000000000000000000000001
near view $ID ft_balance_of '{"account_id": "'es1.$ID'"}'
```
