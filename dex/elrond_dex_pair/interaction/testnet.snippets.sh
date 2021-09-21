PROXY="https://testnet-gateway.elrond.com"
CHAIN_ID="T"

SHARD_0_SC="erd1qqqqqqqqqqqqqpgqtpj57xrqvhrv28cx4px367s75ujknhygydzqfvjwsc"
SHARD_1_SC="erd1qqqqqqqqqqqqqpgqal3apkqdxes5u5tm9fyxpqrgx8mxvt43vejsu66wml"
SHARD_2_SC="erd1qqqqqqqqqqqqqpgqqnk346q2l335jcwdujkr3zkqdtphvsgkh8dq0e2d8j"

first_token_id_str="FIRST-2cdb98"
second_token_id_str="SECOND-f1339e"
first_token_id_hex="0x$(echo -n $first_token_id_str | xxd -p -u | tr -d '\n')"
second_token_id_hex="0x$(echo -n $second_token_id_str | xxd -p -u | tr -d '\n')"
    
deployPair() {
    router_address="0x$(erdpy wallet pem-address-hex $1 | tr -d '\n')"
    router_owner_address=$router_address
    total_fee_percent=300
    special_fee_percent=100

    erdpy --verbose contract deploy --recall-nonce \
        --pem=$1 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=0 \
        --bytecode "../output/elrond_dex_pair.wasm" \
        --arguments \
        ${first_token_id_hex} \
        ${second_token_id_hex} \
        ${router_address} \
        ${router_owner_address} \
        ${total_fee_percent} \
        ${special_fee_percent} \
        --send || return
}

transferFirstTokens() {
    method_name="0x$(echo -n 'acceptEsdtPayment' | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=30000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${first_token_id_hex} $3 ${method_name} \
        --send || return
}

transferSecondTokens() {
    method_name="0x$(echo -n 'acceptEsdtPayment' | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=30000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${second_token_id_hex} $3 ${method_name} \
        --send || return
}

addLiquidity() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=addLiquidity \
        --arguments $3 $4 $5 $6 \
        --send || return
}

setLpTokenIdentifier() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=setLpTokenIdentifier \
        --arguments $3 \
        --send || return
}

setLpTokenIdentifier() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=setLpTokenIdentifier \
        --arguments $3 \
        --send || return
}

getLiquidity() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=30000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=getLiquidity \
        --send || return
}

getClones() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=getClones \
        --send || return
}

addClone() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=addClone \
        --arguments $3 \
        --send || return
}

getReceivedInfo() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=getReceivedInfo \
        --send || return
}

shareInformation() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=$2 \
        --gas-limit=1000000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=shareInformation \
        --send || return
}
