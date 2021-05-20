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
#   $2 = farm contract to receive fees,
#   $3 = farm contract expected token
setFeeOn() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    farm_contract="0x$(erdpy wallet bech32 --decode $2)"
    farm_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $ROUTE_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=200000000 \
        --function=setFeeOn \
        --arguments $pair_address $farm_contract $farm_token \
        --send || return
}

#params:
#   $1 = pair contract to send fees,
#   $2 = farm contract to receive fees,
#   $3 = farm contract expected token
setFeeOff() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    farm_contract="0x$(erdpy wallet bech32 --decode $2)"
    farm_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $ROUTE_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=200000000 \
        --function=setFeeOff \
        --arguments $pair_address $farm_contract $farm_token \
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

# params
#   $1 = Pair Address
setState() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=setState \
        --arguments 0x01 \
        --send || return
}

#### FARM ####

# params:
#   $1 = Farm Pool Token Identifier (Rewards)
#   $2 = Accepted Farming Token Identifier (Farming Token)
#   $3 = Locked Asset Factory Address
deployFarmContract() {
    router_address="0x$(erdpy wallet bech32 --decode $ROUTE_ADDRESS)"
    farmed_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    farming_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    locked_asset_factory_address="0x$(erdpy wallet bech32 --decode $3)"

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../elrond_dex_farm/output/elrond_dex_farm.wasm" \
        --arguments $router_address $farmed_token $farming_token $locked_asset_factory_address \
        --outfile="deploy-farm-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-farm-internal.interaction.json" --expression="data['emitted_tx']['address']")

    erdpy data store --key=address-devnet --value=${ADDRESS}

    echo ""
    echo "Farm Smart Contract address: ${ADDRESS}"
}

# params:
#   $1 = Farm Pool Token Identifier (Rewards)
#   $2 = Accepted Farming Token Identifier (Farming Token)
#   $3 = Locked Asset Factory Address
#   $4 = Farm Address to upgrade
upgradeFarmContract() {
    router_address="0x$(erdpy wallet bech32 --decode $ROUTE_ADDRESS)"
    farmed_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    farming_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    locked_asset_factory_address="0x$(erdpy wallet bech32 --decode $3)"

    erdpy --verbose contract upgrade $4 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../elrond_dex_farm/output/elrond_dex_farm.wasm" \
        --arguments $router_address $farmed_token $farming_token $locked_asset_factory_address \
        --outfile="upgrade-farm-internal.interaction.json" --send || return

    echo ""
    echo "Farm Smart Contract upgraded"
}

# params:
#   $1 = farm contract,
#   $2 = farm token name,
#   $3 = farm token ticker
issueFarmToken() {
    farm_token_name="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    farm_token_ticker="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=issueFarmToken \
        --arguments $farm_token_name $farm_token_ticker \
        --send || return
}

# params:
#   $1 = farm contract
setLocalRolesFarmToken() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=setLocalRolesFarmToken \
          --send || return
}

# params:
#   $1 = farm contract
#   $2 = Pair Address
#   $3 = LPToken Identifier for Pair Address
addAcceptedPairAddressAndLpToken() {
    pair_address="0x$(erdpy wallet bech32 --decode $2)"
    lptoken_identifier="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=addAcceptedPairAddressAndLpToken \
          --arguments $pair_address $lptoken_identifier \
          --send || return

}

#params:
#   $1 = farm contract,
#   $2 = lp token id,
#   $3 = lp token amount in hex
enterFarm() {
    method_name="0x$(echo -n 'enterFarm' | xxd -p -u | tr -d '\n')"
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
#   $1 = farm contract,
#   $2 = lp token id,
#   $3 = lp token amount in hex
enterFarmAndLockRewards() {
    method_name="0x$(echo -n 'enterFarmAndLockRewards' | xxd -p -u | tr -d '\n')"
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
#   $1 = farm token id,
#   $2 = farm token nonce in hex,
#   $3 = farm token amount in hex,
#   $4 = address of staking contract
exitFarm() {
    method_name="0x$(echo -n 'exitFarm' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    stake_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    farm_contract="0x$(erdpy wallet bech32 --decode $4)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=ESDTNFTTransfer \
        --arguments $stake_token $2 $3 $farm_contract $method_name \
        --send || return
}

# params
#   $1 = Farm Address
#   $2 = PerBlockRewards in hex
setPerBlockRewardAmount() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEFAULT_GAS_LIMIT} \
        --function=setPerBlockRewardAmount \
        --arguments $2 \
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
#   $1 = farm Contract Address
getFarmTokenIdentifier() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getFarmTokenId || return
}

# params:
#   $1 = Pair Address,
#   $2 = First Token Identifier,
#   $3 = Second Token Identifier
getReserves() {
    first_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getReserve \
        --arguments $first_token
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getReserve \
        --arguments $second_token || return
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
#   $2 = Token Out Identifier,
#   $3 = Token Out Amount in hex
getAmountIn() {
    token_in="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getAmountIn \
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

getPerBlockRewardAmount() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getPerBlockRewardAmount || return
}

# params
#   $1 = Farm Address
getTotalSupply() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getTotalSupply || return

}

# params:
#   $1 = farm contract,
#   $2 = farm token amount in hex
#   $3 = farm attributes in hex
calculateRewardsForGivenPosition() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=calculateRewardsForGivenPosition \
        --arguments $2 $3 || return
}

getState() {
    erdpy --verbose contract query $1 \
        --proxy=${PROXY} \
        --function=getState || return
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

# params:
#   $1 = Contract Address
pauseContract() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --function=pause \
        --send || return
}

# params:
#   $1 = Contract Address
resumeContract() {
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --function=resume \
        --send || return
}
