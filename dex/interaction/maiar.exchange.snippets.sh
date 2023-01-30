#!/bin/bash

WALLET_PEM="~/Documents/shared_folder/elrond_testnet_wallet.pem"
PROXY="https://testnet-gateway.multiversx.com"
CHAIN_ID="T"
ROUTE_ADDRESS="erd1qqqqqqqqqqqqqpgqnqf6qpnd7y3m6wpkur9u8hg37rvhn5ae0n4se7lw39"
ESDT_ISSUE_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"
DEFAULT_GAS_LIMIT=50000000

##### ENDPOINTS #####

# params:
#   $1 = Token Name
#   $2 = Token Ticker
issueToken() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    initial_supply=0x05F5E100
    token_decimals=0x12

    erdpy --verbose contract call ${ESDT_ISSUE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=5000000000000000000 \
        --function="issue" \
        --arguments ${token_name} ${token_ticker} ${initial_supply} ${token_decimals} \
        --send || return
}

# params:
#   $1 = Pair Address,
#   $2 = First Token Identifier,
#   $3 = First Token Amount in hex,
#   $4 = Second Token Identifier,
#   $5 = Second Token Amount in hex,
#   $6 = Minimum First Token Amount in hex,
#   $7 = Minimum Second Token Amount in hex
addLiquidity() {
    method_name="0x$(echo -n 'acceptEsdtPayment' | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=30000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="multiESDTNFTTransfer" 2 $2 0 $3 $4 0 $5 ${method_name} $6 $7 \
        --arguments $2 $3 $4 $5 \
        --send || return
}

# params:
#   $1 = Pair Address,
#   $2 = LP Token Identifier,
#   $3 = LP Token Amount in hex,
#   $4 = First Token Amount min in hex,
#   $5 = Second Token Amount min in hex
removeLiquidity() {
    method_name="0x$(echo -n 'removeLiquidity' | xxd -p -u | tr -d '\n')"
    lp_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=25000000
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function="ESDTTransfer" \
      --arguments $lp_token $3 $method_name $4 $5 \
      --send || return
}

# params:
#   $1 = Pair Address,
#   $2 = Token In Identifier,
#   $3 = Amount In in hex,
#   $4 = Token Out Identifier,
#   $5 = Amount Out min in hex
swapFixedInput() {
    method_name="0x$(echo -n 'swapTokensFixedInput' | xxd -p -u | tr -d '\n')"
    token_in="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    token_out="0x$(echo -n $4 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=${DEFAULT_GAS_LIMIT} \
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function="ESDTTransfer" \
      --arguments $token_in $3 $method_name $token_out $5 \
      --send || return
}

# params
#   $1 = destination pair contract,
#   $2 = pair contract to be whitelisted.
whitelist() {
    pair_address="0x$(erdpy wallet bech32 --decode $2)"
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=whitelist \
        --arguments $pair_address \
        --send || return
}

# params
#   $1 = destination pair contract,
#   $2 = pair contract which will be added as trusted swap pair,
#   $3 = Trusted Pair First Token Identifier,
#   $4 = Trusted Pair Second Token Identifier,
addTrustedSwapPair() {
    pair_address="0x$(erdpy wallet bech32 --decode $2)"
    first_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $4 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=addTrustedSwapPair \
        --arguments $pair_address $first_token $second_token \
        --send || return
}

#params:
#   $1 = pair contract to send fees,
#   $2 = staking contract to receive fees,
#   $3 = staking contract expected token
setFeeOn() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    staking_contract="0x$(erdpy wallet bech32 --decode $2)"
    staking_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $ROUTE_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=setFeeOn \
        --arguments $pair_address $staking_contract $staking_token \
        --send || return
}

#params:
#   $1 = staking contract,
#   $2 = lp token id,
#   $3 = lp token amount in hex
stake() {
    method_name="0x$(echo -n 'stake' | xxd -p -u | tr -d '\n')"
    staking_contract="0x$(erdpy wallet bech32 --decode $2)"
    lp_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=ESDTTransfer \
        --arguments $staking_contract $lp_token $method_name \
        --send || return
}

#params:
#   $1 = stake token id,
#   $2 = stake token nonce in hex,
#   $3 = stake token amount in hex,
#   $4 = address of staking contract
unstake() {
    method_name="0x$(echo -n 'unstake' | xxd -p -u | tr -d '\n')"
    user_address="0x$(erdpy wallet pem-address $WALLET_PEM)"
    staking_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    staking_contract="0x$(erdpy wallet bech32 --decode $4)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=ESDTNFTTransfer \
        --arguments $staking_token $2 $3 $staking_contract $method_name \
        --send || return
}

# params:
#   $1 = unstake token id,
#   $2 = unstake token nonce in hex,
#   $3 = unstake token amount in hex,
#   $4 = address of staking contract
unbond() {
    method_name="0x$(echo -n 'unbond' | xxd -p -u | tr -d '\n')"
    user_address="0x$(erdpy wallet pem-address $WALLET_PEM)"
    unstaking_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    staking_contract="0x$(erdpy wallet bech32 --decode $4)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=ESDTNFTTransfer \
        --arguments $unstaking_token $2 $3 $staking_contract $method_name \
        --send || return
}

##### VIEW FUNCTIONS #####

# params:
#   $1 = First Token Identifier,
#   $2 = Second Token Identifier
getPairAddress() {
    first_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract query ${ROUTE_ADDRESS} \
        --proxy=${PROXY} \
        --function=getPair \
        --arguments $first_token $second_token || return 
}

# params:
#   $1 = Pair Address
getLpTokenIdentifier() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getLpTokenIdentifier || return
}

# params:
#   $1 = Pair Address,
#   $2 = Token In Identifier,
#   $3 = Token In Amount in hex
getEquivalent() {
    token_in="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getEquivalent \
        --arguments $token_in $3 || return
}

# params:
#   $1 = Pair Address,
#   $2 = Token In Identifier,
#   $3 = Token In Amount in hex
getAmountOut() {
    token_in="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getAmountOut \
        --arguments $token_in $3 || return
}

# params:
#   $1 = Pair Address,
#   $2 = Liquidity amount in hex
getTokensForGivenPosition() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getTokensForGivenPosition \
        --arguments $2 || return
}

# params:
#   $1 = staking contract,
#   $2 = stake token nonce in hex,
#   $3 = stake token amount in hex
calculateRewardsForGivenPosition() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=calculateRewardsForGivenPosition \
        --arguments $2 $3 || return 
}
