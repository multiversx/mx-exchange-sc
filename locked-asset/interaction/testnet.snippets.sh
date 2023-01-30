WALLET_PEM="~/Documents/shared_folder/elrond_testnet_wallet.pem"
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-devnet)
DEPLOY_GAS="1000000000"
PROXY="https://testnet-gateway.multiversx.com"
CHAIN_ID="T"


LOCKED_ASSET_FACTORY_ADDRESS="erd1qqqqqqqqqqqqqpgqhpu9gztn7tmznxc46q46uk6f23ttnern0n4sar7l79"
PROXY_ADDRESS="erd1qqqqqqqqqqqqqpgqdhtgx8pvmjtzkdpykjl570dll33sasze0n4sy76g6e"
DISTRIBUTION_ADDRESS="erd1qqqqqqqqqqqqqpgqc8relmv973g34dytgd55p3hmaaljvka30n4s6n64q6"
DEFAULT_GAS_LIMIT=50000000


##### ENDPOINTS #####

#params:
#   $1 = User Address
#   $2 = Token id,
#   $3 = Token nonce in hex,
#   $4 = Token amount in hex,
transferNFTTokens() {
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="$(echo -n $2 | xxd -p -u | tr -d '\n')"
    address_to_send="$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose tx new --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=100000000 \
        --receiver=$user_address \
        --data "ESDTNFTTransfer@$sft_token@$3@$4@$address_to_send" \
        --send || return
}
#### LOCKED ASSET ####

# params
#   $1 = Asset Token Identifier
deployLockedAssetContract() {
    token_identifier="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract deploy --recall-nonce \
          --pem=${WALLET_PEM} \
          --gas-price=1499999999 \
          --gas-limit=1499999999 \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --bytecode="../../locked-asset/factory/output/factory.wasm" \
          --arguments $token_identifier 0x000000000000000564 \
          --outfile="deploy-locked-asset-internal.interaction.json" --send || return
    
    ADDRESS=$(erdpy data parse --file="deploy-locked-asset-internal.interaction.json" --expression="data['contractAddress']")

    echo ""
    echo "Locked Asset Factory contract address: ${ADDRESS}"
}

# params:
#   $1 = Asset Token Identifier
upgradeLockedAssetContract() {
    token_identifier="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract upgrade $LOCKED_ASSET_FACTORY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../../locked-asset/factory/output/factory.wasm" \
        --arguments $token_identifier 0x000000000000017C64 \
        --outfile="upgrade-locked-asset-internal.interaction.json" --send || return

    echo ""
    echo "Locked Asset Smart Contract upgraded"
}

# params:
#   $1 = Locked Asset Token Name,
#   $2 = Locked Asset Token Ticker,
#   $3 = num decimals,
registerLockedAssetToken() {
    lp_token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    lp_token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call ${LOCKED_ASSET_FACTORY_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=200000000 \
          --value=5000000000000000000 \
          --function=registerLockedAssetToken \
	      --arguments $lp_token_name $lp_token_ticker $3 \
	      --send || return
}

# params:
#   $1 = Address,
setLocalRolesLockedAssetToken() {
    decoded_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call ${LOCKED_ASSET_FACTORY_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=setLocalRolesLockedAssetToken \
        --arguments $decoded_address 0x03 0x04 0x05 \
        --send || return
}

# params
#   $1 = Address
whitelist() {
    sc_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call ${LOCKED_ASSET_FACTORY_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=whitelist \
        --arguments $sc_address \
        --send || return
}

# params
#   $1 = Locked Asset Token Identifier
setLockedAssetTokenId() {
    locked_asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call ${LOCKED_ASSET_FACTORY_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=set_locked_asset_token_id \
        --arguments $locked_asset_token \
        --send || return
}


#### PROXY ####

# params:
#   $1 = Asset Token Identifier
#   $2 = Locked Asset Token Identifier
deployProxyContract() {
    asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    locked_asset_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../proxy_dex/output/proxy_dex.wasm" \
        --arguments $asset_token $locked_asset_token \
        --outfile="deploy-proxy-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-proxy-internal.interaction.json" --expression="data['contractAddress']")

    echo ""
    echo "Proxy Smart Contract address: ${ADDRESS}"
}

# params:
#   $1 = Asset Token Identifier
#   $2 = Locked Asset Token Identifier
upgradeProxyContract() {
    asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    locked_asset_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract upgrade $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=300000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../proxy_dex/output/proxy_dex.wasm" \
        --arguments $asset_token $locked_asset_token \
        --outfile="upgrade-proxy-internal.interaction.json" --send || return

    echo ""
    echo "Upgrade Proxy Smart Contract"
}

# params:
#   $1 = WLPToken token name,
#   $2 = WLPToken token ticker,
#   $3 = num decimals,
registerProxyPair() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=registerProxyPair \
        --arguments $token_name $token_ticker $3 \
        --send || return
}

# params:
#   $1 = WLPToken token name,
#   $2 = WLPToken token ticker,
#   $3 = num decimals,
registerProxyFarm() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=registerProxyFarm \
        --arguments $token_name $token_ticker \
        --send || return
}

# params:
#   $1 = Token Identifier
#   $2 = Address
setLocalRolesProxy() {
    token_identifier="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    address="0x$(erdpy wallet bech32 --decode $2)"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=setLocalRoles \
          --arguments $token_identifier $address 0x05 \
          --send || return
}

# params
#   $1 = Proxy Params in hex
setProxyPairParams() {
    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=setProxyPairParams \
          --arguments $1 \
          --send || return
}

# params
#   $1 = Pair Address
addPairToIntermediate() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=addPairToIntermediate \
          --arguments $pair_address \
          --send || return
}

removeIntermediatedPair() {
    pair_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=removeIntermediatedPair \
          --arguments $pair_address \
          --send || return
}

# params
#   $1 = Farm Address
addFarmToIntermediate() {
    farm_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=addFarmToIntermediate \
          --arguments $farm_address \
          --send || return
}


# params:
#   $1 = Pair Address,
#   $2 = Token Identifier,
#   $3 = Token Amount in hex
transferESDTTokensProxy() {
    method_name="0x$(echo -n 'acceptEsdtPaymentProxy' | xxd -p -u | tr -d '\n')"
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    token_identifier="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=50000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${token_identifier} $3 ${method_name} $pair_address \
        --send || return
}

#params:
#   $1 = Pair Address
#   $2 = Token id,
#   $3 = Token nonce in hex,
#   $4 = Token amount in hex,
transferNFTTokensProxy() {
    method_name="0x$(echo -n 'acceptEsdtPaymentProxy' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    proxy_address="0x$(erdpy wallet bech32 --decode $PROXY_ADDRESS)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=100000000 \
        --function=ESDTNFTTransfer \
        --arguments $sft_token $3 $4 $proxy_address $method_name $pair_address \
        --send || return
}

# params
#   $1 = Pair Address
#   $2 = First Token ID
#   $3 = First Token Nonce
#   $4 = First amount Desired in hex
#   $5 = First amount min in hex
#   $6 = Second Token ID
#   $7 = Second Token Nonce
#   $8 = Second amount desired in hex
#   $9 = Second amount min in hex
addLiquidityProxy() {
    first_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $6 | xxd -p -u | tr -d '\n')"
    pair_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=50000000 \
        --function=addLiquidityProxy \
        --arguments $pair_address $first_token $3 $4 $5 $second_token $7 $8 $9 \
        --send || return
}

# params
#   $1 = Pair Address
#   $2 = WLPToken Identifier
#   $3 = WLPToken Nonce
#   $4 = WLPToken Amount in hex
#   $5 = First Token Amount Min in hex
#   $6 = Second Token Amount Min in hex
removeLiquidityProxy() {
    method_name="0x$(echo -n 'removeLiquidityProxy' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    pair_address="0x$(erdpy wallet bech32 --decode $1)"
    proxy_address="0x$(erdpy wallet bech32 --decode $PROXY_ADDRESS)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=500000000 \
        --function=ESDTNFTTransfer \
        --arguments $sft_token $3 $4 $proxy_address $method_name $pair_address $5 $6 \
        --send || return
}

# params
#   $1 = Farm Address
#   $2 = WLPToken Identifier
#   $3 = WLPToken Nonce
#   $4 = WLPToken Amount in hex
enterFarmProxy() {
    method_name="0x$(echo -n 'enterFarmProxy' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    farm_address="0x$(erdpy wallet bech32 --decode $1)"
    proxy_address="0x$(erdpy wallet bech32 --decode $PROXY_ADDRESS)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=700000000 \
        --function=ESDTNFTTransfer \
        --arguments $sft_token $3 $4 $proxy_address $method_name $farm_address \
        --send || return
}

# params
#   $1 = Farm Address
#   $2 = WLPToken Identifier
#   $3 = WLPToken Nonce
#   $4 = WLPToken Amount in hex
exitFarmProxy() {
    method_name="0x$(echo -n 'exitFarmProxy' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    farm_address="0x$(erdpy wallet bech32 --decode $1)"
    proxy_address="0x$(erdpy wallet bech32 --decode $PROXY_ADDRESS)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=700000000 \
        --function=ESDTNFTTransfer \
        --arguments $sft_token $3 $4 $proxy_address $method_name $farm_address \
        --send || return
}

# params
#   $1 = Farm Address
#   $2 = WLPToken Identifier
#   $3 = WLPToken Nonce
#   $4 = WLPToken Amount in hex
enterFarmAndLockRewardsProxy() {
    method_name="0x$(echo -n 'enterFarmAndLockRewardsProxy' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    farm_address="0x$(erdpy wallet bech32 --decode $1)"
    proxy_address="0x$(erdpy wallet bech32 --decode $PROXY_ADDRESS)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=700000000 \
        --function=ESDTNFTTransfer \
        --arguments $sft_token $3 $4 $proxy_address $method_name $farm_address \
        --send || return
}


# params
#   $1 = Farm Address
#   $2 = WLPToken Identifier
#   $3 = WLPToken Nonce
#   $4 = WLPToken Amount in hex
claimRewardsProxy() {
    method_name="0x$(echo -n 'claimRewardsProxy' | xxd -p -u | tr -d '\n')"
    user_address="$(erdpy wallet pem-address $WALLET_PEM)"
    sft_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    farm_address="0x$(erdpy wallet bech32 --decode $1)"
    proxy_address="0x$(erdpy wallet bech32 --decode $PROXY_ADDRESS)"

    erdpy --verbose contract call $user_address --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=700000000 \
        --function=ESDTNFTTransfer \
        --arguments $sft_token $3 $4 $proxy_address $method_name $farm_address \
        --send || return
}

# params
#   $1 = First Token ID
#   $2 = First Token Nonce
#   $3 = Second Token ID
#   $4 = Second Token Nonce
reclaimTemporaryFundsProxy() {
    first_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    second_token="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=500000000 \
        --function=reclaimTemporaryFundsProxy \
        --arguments $first_token $2 $second_token $4 \
        --send || return
}

#### DISTRIBUTION ####

# params:
#   $1 = Asset Token Identifier
deployDistributionContract() {
    locked_asset_address="0x$(erdpy wallet bech32 --decode $LOCKED_ASSET_FACTORY_ADDRESS)"
    asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../distribution/output/distribution.wasm" \
        --arguments $asset_token $locked_asset_address \
        --outfile="deploy-distribution-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-distribution-internal.interaction.json" --expression="data['contractAddress']")

    echo ""
    echo "Distribution Smart Contract address: ${ADDRESS}"
}

# params:
#   $1 = Asset Token Identifier
upgradeDistributionContract() {
    locked_asset_address="0x$(erdpy wallet bech32 --decode $LOCKED_ASSET_FACTORY_ADDRESS)"
    asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract upgrade $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --bytecode="../distribution/output/distribution.wasm" \
        --arguments $asset_token $locked_asset_address \
        --outfile="upgrade-distribution-internal.interaction.json" --send || return

    echo ""
    echo "Distribution Smart Contract upgraded"
}

startGlobalOperation() {
    erdpy --verbose contract call $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=20000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=startGlobalOperation \
        --send || return
}

endGlobalOperation() {
    erdpy --verbose contract call $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=20000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=endGlobalOperation \
        --send || return
}

setCommunityDistribution() {
    erdpy --verbose contract call $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=50000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=setCommunityDistribution \
        --arguments 0x084595161401484A000000 0x2E \
            0x00000000000000380a \
            0x00000000000000420a \
            0x000000000000004C0a \
            0x00000000000000600a \
            0x00000000000000650a \
            0x000000000000006A0a \
            0x00000000000000740a \
            0x000000000000007E0a \
            0x00000000000000880a \
            0x00000000000000920a \
        --send || return
}


undoLastCommunityDistribution() {
    erdpy --verbose contract call $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=50000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=undoLastCommunityDistribution \
        --send || return
}

# params
#   $1 = Spread Epoch in hex
#   $2 = User Address
#   $3 = Distributed user amount in hex
setPerUserDistributedLockedAssets() {
    user_address="0x$(erdpy wallet bech32 --decode $2)"
    erdpy --verbose contract call $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=50000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=setPerUserDistributedLockedAssets \
        --arguments $1 $user_address $3 \
        --send || return
}

claimLockedAssets() {
    erdpy --verbose contract call $DISTRIBUTION_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=100000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=claimLockedAssets \
        --send || return

}

##### VIEW FUNCTIONS #####

# params
#   $1 = User Address
getTemporaryFundsProxy() {
    user_address="0x$(erdpy wallet bech32 --decode $1)"
    asset_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract query $PROXY_ADDRESS \
        --proxy=${PROXY} \
        --function=getTemporaryFunds \
        --arguments $user_address || return 

}

# params
getIntermediatedFarms() {
    erdpy --verbose contract query $PROXY_ADDRESS \
        --proxy=${PROXY} \
        --function=getIntermediatedFarms || return 

}

# params
getIntermediatedPairs() {
        erdpy --verbose contract query $PROXY_ADDRESS \
        --proxy=${PROXY} \
        --function=getIntermediatedPairs || return 

}

# params:
getLockedTokenId() {
    erdpy --verbose contract query $LOCKED_ASSET_FACTORY_ADDRESS \
        --proxy=${PROXY} \
        --function=getLockedTokenId || return 
}

# params:
getWrappedLpTokenId() {
    erdpy --verbose contract query $PROXY_ADDRESS \
        --proxy=${PROXY} \
        --function=getWrappedLpTokenId || return 
}

# params:
getWrappedFarmTokenId() {
    erdpy --verbose contract query $PROXY_ADDRESS \
        --proxy=${PROXY} \
        --function=getWrappedFarmTokenId || return 
}

getLastCommunityDistributionUnlockMilestones() {
    erdpy --verbose contract query $DISTRIBUTION_ADDRESS \
        --proxy=${PROXY} \
        --function=getLastCommunityDistributionUnlockMilestones || return 

}

getWhitelistedContracts() {
    erdpy --verbose contract query $LOCKED_ASSET_FACTORY_ADDRESS \
        --proxy=${PROXY} \
        --function=getWhitelistedContracts || return 


}

# params
#   $1 = User Address
calculateAssets() {
    user_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract query $DISTRIBUTION_ADDRESS \
        --proxy=${PROXY} \
        --function=calculateAssets \
        --arguments $user_address || return
}

# params
#   $1 = User Address
calculateLockedAssets() {
    user_address="0x$(erdpy wallet bech32 --decode $1)"

    erdpy --verbose contract query $DISTRIBUTION_ADDRESS \
        --proxy=${PROXY} \
        --function=calculateLockedAssets \
        --arguments $user_address || return
}

##### UTILS #####
