### Deployment Setup Steps ###


# Deploy RIDE Proxy Staking Farm from MEX rewards
# $ deployProxyStakeFarmContract $LP_FARM_MEX_ADDRESS $STAKING_FARM_ADDRESS $LP_PAIR_ADDRESS $STAKING_TOKEN_ID $LP_FARM_MEX_TOKEN_ID $STAKING_FARM_TOKEN_ID $LP_TOKEN_ID

# Deploy RIDE Proxy Staking Farm from LKMEX rewards
# $ deployProxyStakeFarmContract $LP_FARM_LKMEX_ADDRESS $STAKING_FARM_ADDRESS $LP_PAIR_ADDRESS $STAKING_TOKEN_ID $LP_FARM_LKMEX_TOKEN_ID $STAKING_FARM_TOKEN_ID $LP_TOKEN_ID

# Issue DualYield Tokens
# $ registerDualYieldToken $PROXY_STAKING_FARM_MEX MetaStakedRide METARIDE 18
# $ registerDualYieldToken $PROXY_STAKING_FARM_LKMEX MetaStakedRideLK METARIDELK 18

# Set local roles for DualYield Tokens
# $ setLocalRolesDualYieldToken $PROXY_STAKING_FARM_MEX
# $ setLocalRolesDualYieldToken $PROXY_STAKING_FARM_LKMEX


WALLET_PEM=""
PROXY="https://devnet-gateway.multiversx.com"
CHAIN_ID="D"


STAKING_TOKEN_ID="" # Fill in token ID
STAKING_FARM_TOKEN_ID="" # Fill in token ID
LP_FARM_MEX_TOKEN_ID="" # Fill in token ID
LP_FARM_LKMEX_TOKEN_ID="" # Fill in token ID
LP_TOKEN_ID="" # Fill in token ID

LP_PAIR_ADDRESS="" # Fill in address
LP_FARM_MEX_ADDRESS="" # Fill in address
LP_FARM_LKMEX_ADDRESS="" # Fill in address
STAKING_FARM_ADDRESS="" # Fill in address

PROXY_STAKING_FARM_MEX="" # Fill in address after deploy
PROXY_STAKING_FARM_LKMEX="" # Fill in address after deploy


# params:
#   $1 = LP Farm Address
#   $2 = Staking Farm Address
#   $3 = Pair Address
#   $4 = Staking Token Identifier
#   $5 = LP Farm Token Identifier
#   $6 = Staking Farm Token Identifier
#   $7 = LP Token Identifier
deployProxyStakeFarmContract() {
    lp_farm_address="0x$(erdpy wallet bech32 --decode $1)"
    staking_farm_address="0x$(erdpy wallet bech32 --decode $2)"
    pair_address="0x$(erdpy wallet bech32 --decode $3)"
    staking_token="0x$(echo -n $4 | xxd -p -u | tr -d '\n')"
    lp_farm_token="0x$(echo -n $5 | xxd -p -u | tr -d '\n')"
    staking_farm_token="0x$(echo -n $6 | xxd -p -u | tr -d '\n')"
    lp_token="0x$(echo -n $7 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract deploy --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=250000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../../sc-dex-rs/dex/farm-staking-proxy/output/farm-staking-proxy.wasm" \
        --arguments $lp_farm_address $staking_farm_address $pair_address $staking_token $lp_farm_token $staking_farm_token $lp_token\
        --outfile="deploy-proxy-stake-farm-internal.interaction.json" --send || return

    ADDRESS=$(erdpy data parse --file="deploy-proxy-stake-farm-internal.interaction.json" --expression="data['contractAddress']")

    echo ""
    echo "Metastaking Smart Contract address: ${ADDRESS}"
}


# params:
#   $1 = LP Farm Address
#   $2 = Staking Farm Address
#   $3 = Pair Address
#   $4 = Staking Token Identifier
#   $5 = LP Farm Token Identifier
#   $6 = Staking Farm Token Identifier
#   $7 = Staking Farm Token Identifier
#   $8 = Staking Proxy Address
upgradeProxyStakeFarmContract() {
    lp_farm_address="0x$(erdpy wallet bech32 --decode $1)"
    staking_farm_address="0x$(erdpy wallet bech32 --decode $2)"
    pair_address="0x$(erdpy wallet bech32 --decode $3)"
    staking_token="0x$(echo -n $4 | xxd -p -u | tr -d '\n')"
    lp_farm_token="0x$(echo -n $5 | xxd -p -u | tr -d '\n')"
    staking_farm_token="0x$(echo -n $6 | xxd -p -u | tr -d '\n')"
    lp_token="0x$(echo -n $7 | xxd -p -u | tr -d '\n')"

    erdpy --verbose contract upgrade $8 --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --metadata-payable \
        --bytecode="../../sc-dex-rs/dex/farm-staking-proxy/output/farm-staking-proxy.wasm" \
        --arguments $lp_farm_address $staking_farm_address $pair_address $staking_token $lp_farm_token $staking_farm_token $lp_token \
        --outfile="deploy-proxy-stake-farm-internal.interaction.json" --send || return
}


# params:
#   $1 = Proxy Staking Farm Address,
#   $2 = Proxy Staking Farm Token name,
#   $3 = Proxy Staking Farm Token ticker,
#   $3 = Proxy Staking Farm Token num decimals,
registerDualYieldToken() {
    farm_token_name="0x$(echo -n $2 | xxd -p -u | tr -d '\n')"
    farm_token_ticker="0x$(echo -n $3 | xxd -p -u | tr -d '\n')"
    erdpy --verbose contract call $1 --recall-nonce \
        --pem=${WALLET_PEM} \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --gas-limit=100000000 \
        --value=50000000000000000 \
        --function=registerDualYieldToken \
        --arguments $farm_token_name $farm_token_ticker $4 \
        --send || return
}

# params:
#   $1 = farm contract
setLocalRolesDualYieldToken() {
    erdpy --verbose contract call $1 --recall-nonce \
          --pem=${WALLET_PEM} \
          --proxy=${PROXY} --chain=${CHAIN_ID} \
          --gas-limit=200000000 \
          --function=setLocalRolesDualYieldToken \
          --send || return
}
