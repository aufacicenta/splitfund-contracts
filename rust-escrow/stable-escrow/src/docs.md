
## Deploy NEP141

npm install -g near-cli
rustup target add wasm32-unknown-unknown
near dev-deploy --wasmFile res/fungible_token.wasm
source neardev/dev-account.env
near call $CONTRACT_NAME new '{"owner_id": "'$CONTRACT_NAME'", "total_supply": "1000000000000000", "metadata": { "spec": "ft-1.0.0", "name": "Example Token Name", "symbol": "EXLT", "decimals": 8 }}' --accountId $CONTRACT_NAME

ID=$CONTRACT_NAME

## Test NEP141
near create-account bob.$ID --masterAccount $ID --initialBalance 1
near call $ID storage_deposit '' --accountId bob.$ID --amount 0.00125
near call $ID ft_transfer '{"receiver_id": "'bob.$ID'", "amount": "100000000"}' --accountId $ID --amount 0.000000000000000000000001
near view $ID ft_balance_of '{"account_id": "'bob.$ID'"}'

#####
# Staking Factory
#####
near delete sf1.$ID $ID
near create-account sf1.$ID --masterAccount $ID --initialBalance 6
near deploy --wasmFile ../src/staking_factory.wasm --accountId sf1.$ID
near call sf1.$ID new '' --accountId sf1.$ID

#####
# Deploy Escrow
#####
near delete es1.$ID $ID
near create-account es1.$ID --masterAccount $ID --initialBalance 5
near deploy --wasmFile ../src/stable_escrow.wasm --accountId es1.$ID
# Vence en diciembre, funding 10,0000
near call es1.$ID new '{"decimals": 2, "expires_at": 1670215945000000000, "funding_amount_limit": "10000", "id": "sa18", "nep_141": "'$ID'", "dao_factory": "sputnikv2.testnet", "maintainer": "'$ID'", "metadata_url": "", "staking_factory": "sf1.'$ID'"}' --accountId $ID

near view es1.$ID ft_balance_of '{"account_id": "'bob.$ID'"}'
near view es1.$ID ft_total_supply
near view es1.$ID ft_metadata

# Depositar
near call $ID storage_deposit '' --accountId es1.$ID --amount 0.00125
near view $ID ft_balance_of '{"account_id": "'es1.$ID'"}'

near call es1.$ID storage_deposit '' --accountId bob.$ID --amount 0.00125
near call $ID ft_transfer_call '{"receiver_id": "'es1.$ID'", "amount": "10000", "msg": ""}' --accountId bob.$ID --amount 0.000000000000000000000001 --gas 50000000000000

near view es1.$ID ft_balance_of '{"account_id": "'bob.$ID'"}'
near view $ID ft_balance_of '{"account_id": "'es1.$ID'"}'
near view es1.$ID get_deposit_accounts

# Retirar
near view es1.$ID is_deposit_allowed
near view es1.$ID is_withdrawal_allowed
near call es1.$ID withdraw --accountId bob.$ID --amount 0.000000000000000000000001

#####
# Create DAO and Stake
#####

export MAX_GAS=300000000000000
near call es1.$ID create_dao '' --accountId $ID --amount 6 --gas $MAX_GAS
near call es1.$ID create_stake '' --accountId $ID --amount 3 --gas $MAX_GAS
near call es1.$ID setup_dao '' --accountId $ID --amount 1 --gas $MAX_GAS

DAO_ACCOUNT_ID=sa18.sputnikv2.testnet
near view $DAO_ACCOUNT_ID get_staking_contract
near view $DAO_ACCOUNT_ID get_policy

#####
# Setup dao
#####

STAKING_ACCOUNT_ID=sa14.sf1.$ID
DAO_ACCOUNT_ID=sa14.sputnikv2.testnet

near call $DAO_ACCOUNT_ID add_proposal '{"proposal": { "description": "", "kind": { "SetStakingContract": { "staking_id": "'$STAKING_ACCOUNT_ID'" } } } }' --accountId $ID --amount 0.1
near call $DAO_ACCOUNT_ID act_proposal '{"id": 0, "action" :"VoteApprove"}' --accountId $ID  --gas $MAX_GAS
near view $DAO_ACCOUNT_ID get_staking_contract

near view $DAO_ACCOUNT_ID get_staking_contract

#####
# Create Stake
#####

near call sf1.$ID create_stake '{"name": "sa6", "dao_account_id": "sa6.sputnikv2.testnet", "token_account_id": "'$ID'", "unstake_period": "604800000000000"}' --accountId $ID --amount 3 --gas $MAX_GAS
