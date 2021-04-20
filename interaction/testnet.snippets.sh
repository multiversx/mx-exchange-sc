# WALLET_PEM="~/Elrond/MySandbox/testnet/wallets/users/alice.pem"
WALLET_PEM="/home/elrond/Test/erd1qa2pdw875gyd3ct05f3npk8wzm9mrxsdtstcc2trfw5aqpqcephqzd6trq.pem"
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-devnet)
DEPLOY_GAS="1000000000"
PROXY="https://testnet-gateway.elrond.com"
CHAIN_ID="T"
# PROXY="http://localhost:7950"
# CHAIN_ID="local-testnet"

ESDT_ISSUE_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"
ROUTE_ADDRESS="erd1qqqqqqqqqqqqqpgqwge8qwlqcq2066phdvqysvnsn9x6xg8uephqxf7ang"
WEGLD_WRAP_ADDRESS="erd1qqqqqqqqqqqqqpgq37e5r67hvtrkyhs6yadwvwtk3rxk792e0n4s066pa5"
DEFAULT_GAS_LIMIT=50000000

##### ENDPOINTS #####

# params:
#   $1 = Token Name
#   $2 = Token Ticker
issueToken() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    initial_supply=0xFFFFFFFFFFFFFFFFFFFF
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

#### ROUTER ####

deployRouterContract() {
    erdpy --verbose contract deploy --recall-nonce \
          --pem=${WALLET_PEM} \
          --gas-price=1499999999 \
          --gas-limit=1499999999 \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --metadata-payable \
          --bytecode="../elrond_dex_router/output/elrond_dex_router.wasm" \
          --outfile="deploy-route-internal.interaction.json" --send || return
    
    ADDRESS=$(erdpy data parse --file="deploy-route-internal.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=router-address --value=${ADDRESS}

    echo ""
    echo "Route Smart contract address: ${ADDRESS}"
}

upgradeRouterContract() {
    erdpy --verbose contract upgrade ${ROUTE_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --gas-limit=${DEPLOY_GAS} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --bytecode="../elrond_dex_router/output/elrond_dex_router.wasm" \
          --metadata-payable \
          --outfile="upgrade-route-internal.interaction.json" --send || return

    echo ""
    echo "Route Smart contract upgraded"
}

uploadPairContractCode() {
    echo "STARTING TO PUSH NEW PAIR CONTRACT"
    PAIR_CODE_HEX="$(xxd -p ../elrond_dex_pair/output/elrond_dex_pair.wasm | tr -d '\n')"
    PAIR_CODE_HEX="${PAIR_CODE_HEX::-4}"
    PAIR_CODE_HEX1="0x$(split -n1/3 <<<$PAIR_CODE_HEX)"
    PAIR_CODE_HEX2="0x$(split -n2/3 <<<$PAIR_CODE_HEX)"
    PAIR_CODE_HEX3="0x$(split -n3/3 <<<$PAIR_CODE_HEX)7575"

    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-price=1400000000 \
          --gas-limit=1400000000 \
          --function=startPairCodeConstruction \
          --send
    sleep 6

    echo "SENDING BATCH 1"
    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-price=1400000000 \
        --gas-limit=1400000000 \
        --function=appendPairCode \
        --arguments $PAIR_CODE_HEX1 \
        --send
    sleep 10

    echo "SENDING BATCH 2"
    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-price=1400000000 \
        --gas-limit=1400000000 \
        --function=appendPairCode \
        --arguments $PAIR_CODE_HEX2 \
        --send
    sleep 10

    echo "SENDING BATCH 3"
    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-price=1400000000 \
        --gas-limit=1400000000 \
        --function=appendPairCode \
        --arguments $PAIR_CODE_HEX3 \
        --send
    sleep 10

    echo "ENDING TO CREATE NEW PAIR"
    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-price=1400000000 \
          --gas-limit=1400000000 \
          --function=endPairCodeConstruction \
          --send
    sleep 6
}

# params:
#   $1 = First Token Identifier,
#   $2 = Second Token Identifier,
createPair() {
    first_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-price=1400000000 \
          --gas-limit=1400000000 \
          --function=createPair \
          --arguments $first_token $second_token \
          --send
    sleep 6

    echo "NEW PAIR CONTRACT ADDRESS:"
    erdpy --verbose contract query ${ROUTE_ADDRESS} \
    --proxy=${PROXY} \
    --function=getPair \
    --arguments $first_token $second_token || return
}

# params:
#   $1 = Pair Address,
issueLpToken() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    lp_token_name="0x$(echo -n 'LPToken' | xxd -p -u | tr -d '\n')"
    lp_token_ticker="0x$(echo -n 'LPT' | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=200000000 \
          --value=5000000000000000000 \
          --function=issueLpToken \
	      --arguments $pair_address $lp_token_name $lp_token_ticker \
	      --send || return
}

# params:
#   $1 = Pair Address,
setLpTokenLocalRoles() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call ${ROUTE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="setLocalRoles" \
        --arguments $pair_address \
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
        --gas-limit=200000000 \
        --function=setFeeOn \
        --arguments $pair_address $staking_contract $staking_token \
        --send || return
}


#### PAIRS ####

# params:
#   $1 = Pair Address,
#   $2 = Token Identifier,
#   $3 = Token Amount in hex
transferTokens() {
    method_name="0x$(echo -n 'acceptEsdtPayment' | xxd -p -u | tr -d '\n')"
    token_identifier="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=20000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${token_identifier} $3 ${method_name} \
        --send || return
}

# params:
#   $1 = Pair Address,
reclaimTemporaryFunds() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=20000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="reclaimTemporaryFunds" \
        --send || return

}

# params:
#   $1 = Pair Address,
#   $2 = First Token Amount in hex,
#   $3 = Second Token Amount in hex,
#   $4 = Minimum First Token Amount in hex,
#   $5 = Minimum Second Token Amount in hex
addLiquidity() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=30000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=addLiquidity \
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
      --gas-limit=25000000 \
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
      --gas-limit=100000000 \
      --proxy=${PROXY} --chain=${CHAIN_ID} \
      --function="ESDTTransfer" \
      --arguments $token_in $3 $method_name $token_out $5 \
      --send || return
}

# params:
#   $1 = Pair Address,
#   $2 = Token In max Identifier,
#   $3 = Amount In in hex,
#   $4 = Token Out Identifier,
#   $5 = Amount Out in hex
swapFixedOutput() {
    method_name="0x$(echo -n 'swapTokensFixedOutput' | xxd -p -u | tr -d '\n')"
    token_in="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    token_out="0x$(echo -n $4 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
      --pem=${WALLET_PEM} \
      --gas-limit=100000000 \
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

#params
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

#### STAKING ####

# params:
#   $1 = Staking Pool Token Identifier
deployStakingContract() {
    staking_pool_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    router_address="0x$(erdpy wallet bech32 --decode $ROUTE_ADDRESS)"
    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../elrond_dex_staking/output/elrond_dex_staking.wasm" \
        --arguments $staking_pool_token $router_address \
        --outfile="deploy-staking-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-staking-internal.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-devnet --value=${ADDRESS}

    echo ""
    echo "Staking Smart contract address: ${ADDRESS}"
}

# params:
#   $1 = staking contract,
#   $2 = stake token name,
#   $3 = stake token ticker
issueStakeToken() {
    stake_token_name="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    stake_token_ticker="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=issueStakeToken \
        --arguments $stake_token_name $stake_token_ticker \
        --send || return
}

# params:
#   $1 = staking contract
setLocalRolesStakeToken() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=setLocalRolesStakeToken \
          --send || return
}

# params:
#   $1 = staking contract,
#   $2 = unstake token name,
#   $3 = unstake token ticker
issueUnstakeToken() {
    unstake_token_name="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    unstake_token_ticker="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --value=5000000000000000000 \
          --function=issueUnstakeToken \
          --arguments $unstake_token_name $unstake_token_ticker \
          --send || return
}

# params:
#   $1 = staking contract
setLocalRolesUnstakeToken() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=setLocalRolesUnstakeToken \
          --send || return
}

#params:
#   $1 = staking contract,
#   $2 = lp token id,
#   $3 = lp token amount in hex
stake() {
    method_name="0x$(echo -n 'stake' | xxd -p -u | tr -d '\n')"
    lp_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=100000000 \
        --function=ESDTTransfer \
        --arguments $lp_token $3 $method_name \
        --send || return
}

#params:
#   $1 = stake token id,
#   $2 = stake token nonce in hex,
#   $3 = stake token amount in hex,
#   $4 = address of staking contract
unstake() {
    method_name="0x$(echo -n 'unstake' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    stake_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    staking_contract="0x$(erdpy wallet bech32 --decode $4)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=ESDTNFTTransfer \
        --arguments $stake_token $2 $3 $staking_contract $method_name \
        --send || return
}

# params:
#   $1 = unstake token id,
#   $2 = unstake token nonce in hex,
#   $3 = unstake token amount in hex,
#   $4 = address of staking contract
unbond() {
    method_name="0x$(echo -n 'unbond' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    unstake_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    staking_contract="0x$(erdpy wallet bech32 --decode $4)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=ESDTNFTTransfer \
        --arguments $unstake_token $2 $3 $staking_contract $method_name \
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
#   $1 = Staking Contract Address
getStakeTokenIdentifier() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getStakeTokenId || return
}

# params:
#   $1 = Staking Contract Address
getUnstakeTokenIdentifier() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getUnstakeTokenId || return
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


##### UTILS #####

deployWEGLDContract() {
    erdpy --verbose contract deploy --recall-nonce \
          --pem=${WALLET_PEM} \
          --gas-limit=100000000 \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --metadata-payable \
          --bytecode="/home/elrond/Elrond/sc-bridge-elrond/egld-esdt-swap/output/egld-esdt-swap.wasm" \
          --outfile="deploy-wegld-internal.interaction.json" --wait-result --send || return
    
    ADDRESS=$(erdpy data parse --file="deploy-wegld-internal.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=router-address --value=${ADDRESS}

    echo ""
    echo "WEGLD Smart contract address: ${ADDRESS}"
}

issueWrappedEgld() {
    erdpy --verbose contract call ${WEGLD_WRAP_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=issueWrappedEgld \
        --arguments 0x5772617070656445474c44 0x5745474c44 10000000 \
        --send || return   
}

setWEGLDLocalRole() {
    erdpy --verbose contract call ${WEGLD_WRAP_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --function=setLocalMintRole \
        --send || return   
}

wrapEgld() {
    erdpy --verbose contract call ${WEGLD_WRAP_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=$1 \
        --function=wrapEgld \
        --send || return   

}

mintWrappedEgld() {
    erdpy --verbose contract call ${WEGLD_WRAP_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --function=mintWrappedEgld \
        --arguments $1 \
        --send || return
}
