### Deployment Setup Steps ###

# Deploy price discovery contract
# $ deployPriceDiscoveryContract

# Register Redeem Token
#  issueRedeemToken DTKEGLDRedeemToken DTKEGLDRT 18

WALLET_PEM=""
PROXY="https://testnet-gateway.multiversx.com"
CHAIN_ID="T"

ZERO_ADDRESS="erd1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq6gq4hu"
ESDT_ISSUE_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"
LOCKING_SC_ADDRESS=""
PRICE_DISCOVERY_ADDRESS=""

LAUNCHED_TOKEN_ID=""
ACCEPTED_TOKEN_ID=""
LAUNCHED_TOKEN_DECIMALS= # decimal format
MIN_LAUNCHED_TOKEN_PRICE= # HEX format
START_BLOCK= # decimal format
NO_LIMIT_PHASE_DURATION_BLOCKS= # decimal format
LINEAR_PENALTY_PHASE_DURATION_BLOCKS= # decimal format
FIXED_PENALTY_PHASE_DURATION_BLOCKS= # decimal format
UNLOCK_EPOCH= # decimal format
# MAX_PERCENTAGE = 10_000_000_000_000;
PENALTY_MIN_PERCENTAGE= # Hex format; ex: 0x174876E800 (100000000000 = 1%)
PENALTY_MAX_PERCENTAGE= # Hex format; ex: 0xE8D4A51000 (1000000000000 = 10%)
FIXED_PENALTY_PERCENTAGE= # Hex format; ex: 0xE8D4A51000 (1000000000000 = 10%)


deployPriceDiscoveryContract() {
    launched_token="0x$(echo -n $LAUNCHED_TOKEN_ID | xxd -p -u | tr -d '\n')"
    accepted_token="0x$(echo -n $ACCEPTED_TOKEN_ID | xxd -p -u | tr -d '\n')"
    locking_address="0x$(erdpy wallet bech32 --decode $LOCKING_SC_ADDRESS)"

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=350000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../output/price-discovery.wasm" \
        --arguments $launched_token \
            $accepted_token \
            $LAUNCHED_TOKEN_DECIMALS \
            $MIN_LAUNCHED_TOKEN_PRICE \
            $START_BLOCK \
            $NO_LIMIT_PHASE_DURATION_BLOCKS \
            $LINEAR_PENALTY_PHASE_DURATION_BLOCKS \
            $FIXED_PENALTY_PHASE_DURATION_BLOCKS \
            $UNLOCK_EPOCH \
            $PENALTY_MIN_PERCENTAGE \
            $PENALTY_MAX_PERCENTAGE \
            $FIXED_PENALTY_PERCENTAGE \
            $locking_address \
        --outfile="deploy-price-discovery.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-price-discovery.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-testnet --value=${ADDRESS}

    echo ""
    echo "Price Discovery Smart Contract address: ${ADDRESS}"
}

upgradePriceDiscoveryContract() {
    launched_token="0x$(echo -n $LAUNCHED_TOKEN_ID | xxd -p -u | tr -d '\n')"
    accepted_token="0x$(echo -n $ACCEPTED_TOKEN_ID | xxd -p -u | tr -d '\n')"
    locking_address="0x$(erdpy wallet bech32 --decode $LOCKING_SC_ADDRESS)"

    erdpy --verbose contract upgrade $PRICE_DISCOVERY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=350000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../output/price-discovery.wasm" \
        --arguments $launched_token \
            $accepted_token \
            $LAUNCHED_TOKEN_DECIMALS \
            $MIN_LAUNCHED_TOKEN_PRICE \
            $START_BLOCK \
            $NO_LIMIT_PHASE_DURATION_BLOCKS \
            $LINEAR_PENALTY_PHASE_DURATION_BLOCKS \
            $FIXED_PENALTY_PHASE_DURATION_BLOCKS \
            $UNLOCK_EPOCH \
            $PENALTY_MIN_PERCENTAGE \
            $PENALTY_MAX_PERCENTAGE \
            $FIXED_PENALTY_PERCENTAGE \
            $locking_address \
        --outfile="upgrade-price-discovery.interaction.json" --send || return

    echo ""
    echo "Price Discovery Smart Contract upgraded"
}

setLockingScAddress() {
    locking_sc_address="0x$(erdpy wallet bech32 --decode $LOCKING_SC_ADDRESS)"

    erdpy --verbose contract call $PRICE_DISCOVERY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="setLockingScAddress" \
        --arguments $locking_sc_address \
        --send || return
}

# params
#   $1 = Token name
#   $2 = Token ticker
#   $3 = Decimals
issueRedeemToken() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $PRICE_DISCOVERY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="issueRedeemToken" \
        --arguments $token_name $token_ticker $3 \
        --send || return
}

createInitialRedeemTokens() {
    erdpy --verbose contract call $PRICE_DISCOVERY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="createInitialRedeemTokens" \
        --send || return
}
