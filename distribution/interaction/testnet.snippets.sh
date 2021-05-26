WALLET_PEM="~/Documents/shared_folder/elrond_testnet_wallet.pem"
DEPLOY_TRANSACTION=$(erdpy data load --key=deployTransaction-devnet)
DEPLOY_GAS="1000000000"
PROXY="https://testnet-gateway.elrond.com"
CHAIN_ID="T"


LOCKED_ASSET_FACTORY_ADDRESS="erd1qqqqqqqqqqqqqpgq5jz83n3ay53e69vgl7vhd7ffg3lheyh40n4spnjeky"
PROXY_ADDRESS="erd1qqqqqqqqqqqqqpgqe50qczp84jdlefhahxfgj3dqlkuafu0q0n4s0pgjmh"
DISTRIBUTION_ADDRESS="erd1qqqqqqqqqqqqqpgqezwr9ln7lf6f9xq8ht62yhclpusscem20n4sj2v99g"
DEFAULT_GAS_LIMIT=50000000


##### ENDPOINTS #####


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
          --bytecode="../sc-locked-asset-factory/output/sc-locked-asset-factory.wasm" \
          --arguments $token_identifier 0x000000000000017C64 \
          --outfile="deploy-locked-asset-internal.interaction.json" --send || return
    
    ADDRESS=$(erdpy data parse --file="deploy-locked-asset-internal.interaction.json" --expression="data['emitted_tx']['address']")

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
        --bytecode="../sc-locked-asset-factory/output/sc-locked-asset-factory.wasm" \
        --arguments $token_identifier 0x000000000000017C64 \
        --outfile="upgrade-locked-asset-internal.interaction.json" --send || return

    echo ""
    echo "Locked Asset Smart Contract upgraded"
}

# params:
#   $1 = Locked Asset Token Name,
#   $2 = Locked Asset Token Ticker
issueNft() {
    lp_token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    lp_token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call ${LOCKED_ASSET_FACTORY_ADDRESS} --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=200000000 \
          --value=5000000000000000000 \
          --function=issueNft \
	      --arguments $lp_token_name $lp_token_ticker \
	      --send || return
}

# params:
#   $1 = Token Identifier
#   $2 = SC Address,
setLocalRolesLockedAsset() {
    token_identifier="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    sc_address="0x$(erdpy wallet bech32 --decode $2)"

    erdpy --verbose contract call ${LOCKED_ASSET_FACTORY_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function=setLocalRoles \
        --arguments $token_identifier $sc_address 0x03 0x04 0x05 \
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

#### PROXY ####

# params:
#   $1 = Asset Token Identifier
deployProxyContract() {
    asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    proxy_farm_params=0x0000000005F5E1000000000005F5E1000000000005F5E10000000000004C4B4000000000004C4B40
    proxy_pair_params=0x0000000005F5E10000000000017D784000000000017D78400000000002625A0000000000004C4B4000000000004C4B40

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../sc-proxy-dex/output/sc-proxy-dex.wasm" \
        --arguments $asset_token $proxy_pair_params $proxy_farm_params \
        --outfile="deploy-proxy-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-proxy-internal.interaction.json" --expression="data['emitted_tx']['address']")

    echo ""
    echo "Proxy Smart Contract address: ${ADDRESS}"
}

# params:
#   $1 = Asset Token Identifier
upgradeProxyContract() {
    asset_token="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    proxy_farm_params=0x0000000005F5E1000000000005F5E1000000000005F5E10000000000004C4B4000000000004C4B40
    proxy_pair_params=0x0000000005F5E10000000000017D784000000000017D78400000000002625A0000000000004C4B4000000000004C4B40

    erdpy --verbose contract upgrade $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-price=1499999999 \
        --gas-limit=1499999999 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../sc-proxy-dex/output/sc-proxy-dex.wasm" \
        --arguments $asset_token $proxy_pair_params $proxy_farm_params \
        --outfile="upgrade-proxy-internal.interaction.json" --send || return

    echo ""
    echo "Upgrade Proxy Smart Contract"
}

# params:
#   $1 = WLPToken token name,
#   $2 = WLPToken token ticker
issueSftProxyPair() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=issueSftProxyPair \
        --arguments $token_name $token_ticker \
        --send || return
}

# params:
#   $1 = WLPToken token name,
#   $2 = WLPToken token ticker
issueSftProxyFarm() {
    token_name="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"
    token_ticker="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=${DEPLOY_GAS} \
        --value=5000000000000000000 \
        --function=issueSftProxyFarm \
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
          --arguments $token_identifier $address 0x03 0x04 0x05 \
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
#   $1 = Token Identifier
addAcceptedLockedAssetTokenId() {
    token_identifier="0x$(echo -n $1 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract call $PROXY_ADDRESS --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=${DEPLOY_GAS} \
          --function=addAcceptedLockedAssetTokenId \
          --arguments $token_identifier \
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
        --gas-limit=500000000 \
        --function=addLiquidityProxy \
        --arguments $pair_address $first_token $3 $4 $5 $second_token $7 $8 $9 \
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
        --bytecode="../sc-distribution-rs/output/sc_distribution_rs.wasm" \
        --arguments $asset_token $locked_asset_address \
        --outfile="deploy-distribution-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-distribution-internal.interaction.json" --expression="data['emitted_tx']['address']")

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
        --bytecode="../sc-distribution-rs/output/sc_distribution_rs.wasm" \
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
        --arguments 0x3635C9ADC5DEA00000 0x018F \
            0x00000000000001900a \
            0x000000000000019A0a \
            0x00000000000001A40a \
            0x00000000000001AE0a \
            0x00000000000001B80a \
            0x00000000000001C20a \
            0x00000000000001CC0a \
            0x00000000000001D60a \
            0x00000000000001E00a \
            0x00000000000001EA0a \
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
#   $2 = Token ID
#   $3 = Token Nonce in hex
getTemporaryFunds() {
    user_address="0x$(erdpy wallet bech32 --decode $1)"
    asset_token="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract query $PROXY_ADDRESS \
        --proxy=${PROXY} \
        --function=getTemporaryFunds \
        --arguments $user_address $asset_token $3  || return 

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
