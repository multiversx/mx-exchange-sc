WALLET_PEM=""
PROXY="https://testnet-gateway.multiversx.com"
CHAIN_ID="T"

ZERO_ADDRESS="erd1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq6gq4hu"
ESDT_ISSUE_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"
SIMPLE_LOCK_ADDRESS=""
PAIR_ADDRESS=""

deploySimpleLockContract() {
    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=150000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../output/simple-lock.wasm" \
        --outfile="deploy-simple-lock.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-simple-lock.interaction.json" --expression="data['emitted_tx']['address']")

    echo ""
    echo "Simple lock Smart Contract address: ${ADDRESS}"
}

upgradeSimpleLockContract() {
    erdpy --verbose contract upgrade $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=150000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../output/simple-lock.wasm" \
        --outfile="upgrade-simple-lock.interaction.json" --send || return
}

# params
#   $1 = Token name
#   $2 = Token ticker
#   $3 = Decimals
issueLockedToken() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="issueLockedToken" \
        --arguments $token_name $token_ticker $3 \
        --send || return
}

setLocalRolesLockedToken() {
    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="setLocalRolesLockedToken" \
        --send || return
}


# params
#   $1 = Token name
#   $2 = Token ticker
#   $3 = Decimals
issueLpProxyToken() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="issueLpProxyToken" \
        --arguments $token_name $token_ticker $3 \
        --send || return
}

setLocalRolesLpProxyToken() {
    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="setLocalRolesLpProxyToken" \
        --send || return
}

# params
#   $1 = Token name
#   $2 = Token ticker
#   $3 = Decimals
issueFarmProxyToken() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="issueFarmProxyToken" \
        --arguments $token_name $token_ticker $3 \
        --send || return
}

setLocalRolesFarmProxyToken() {
    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="setLocalRolesFarmProxyToken" \
        --send || return
}

# params
#   $1 = First Token identifier
#   $2 = Second Token identifier
addLpToWhitelist() {
    first_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    pair_address="0x$(erdpy wallet bech32 --decode $PAIR_ADDRESS)"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=10000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="addLpToWhitelist" \
        --arguments $pair_address $first_token $second_token \
        --send || return
}

# params
#   $1 = First Token identifier
#   $2 = Second Token identifier
removeLpFromWhitelist() {
    first_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    pair_address="0x$(erdpy wallet bech32 --decode $PAIR_ADDRESS)"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=10000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="removeLpFromWhitelist" \
        --arguments $pair_address $first_token $second_token \
        --send || return
}

# params
#   $1 = Farm Address
#   $2 = Farming Token identifier
#   $3 = Farm type
addFarmToWhitelist() {
    farm_address="0x$(erdpy wallet bech32 --decode $1)"
    farming_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=10000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="addFarmToWhitelist" \
        --arguments $farm_address $farming_token $3 \
        --send || return
}

# params
#   $1 = Farm Address
#   $1 = Farming Token identifier
#   $2 = Farm type
removeFarmFromWhitelist() {
    farm_address="0x$(erdpy wallet bech32 --decode $1)"
    farming_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $SIMPLE_LOCK_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=10000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="removeFarmFromWhitelist" \
        --arguments $farm_address $farming_token $3 \
        --send || return
}
