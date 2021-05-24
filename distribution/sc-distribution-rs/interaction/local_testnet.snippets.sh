ALICE="/home/elrond/MySandbox/testnet/wallets/users/alice.pem"
ADDRESS="erd1qqqqqqqqqqqqqpgqfzydqmdw7m2vazsp6u5p95yxz76t2p9rd8ss0zp9ts" #TODO: Change after deploy
ADDRESS_HEX="0x$(erdpy wallet bech32 --decode $ADDRESS)"
PROXY="http://localhost:7950"
CHAIN_ID="local-testnet"

USER_1="erd1qqqqqqqqqqqqqpgqfzydqmdw7m2vazsp6u5p95yxz76t2p9rd8ss0zp9ts"
USER_1_HEX="0x$(erdpy wallet bech32 --decode $USER_1)"
USER_1_REWARD=0x10
USER_2="erd1qyu5wthldzr8wx5c9ucg8kjagg0jfs53s8nr3zpz3hypefsdd8ssycr6th"
USER_2_HEX="0x$(erdpy wallet bech32 --decode $USER_2)"
USER_2_REWARD=0x10

DISTRIBUTED_TOKEN_ID="MEX-abcdef" #TODO: Set after issue
DISTRIBUTED_TOKEN_ID_HEX="0x4d45582d616263646566" #TODO: Set after issue. Don't forget to set LocalMint and LocalBurn to ADDRESS

REWARD_AMOUNT=0x1000
REWARD_EPOCH=10

deploy() {
    erdpy --verbose contract deploy --bytecode="../output/sc_distribution_rs.wasm" --recall-nonce --pem=${ALICE} --gas-limit=900000000 --arguments ${DISTRIBUTED_TOKEN_ID_HEX} --send --outfile="deploy-testnet.interaction.json" --proxy=${PROXY} --chain=${CHAIN_ID} || return
}

startGlobalOperation() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=900000000 --function="startGlobalOperation" --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

setCommunityReward() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=900000000 --function="setCommunityReward" --arguments ${REWARD_AMOUNT} ${REWARD_EPOCH} --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

setPerUserRewards() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=900000000 --function="setPerUserRewards" --arguments ${REWARD_EPOCH} ${USER_1_HEX} ${USER_1_REWARD} ${USER_2_HEX} ${USER_2_REWARD} --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

endGlobalOperation() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=900000000 --function="endGlobalOperation" --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

claimRewards() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=900000000 --function="claimRewards" --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

clearUnclaimedRewards() {
    erdpy --verbose contract call ${ADDRESS} --recall-nonce --pem=${ALICE} --gas-limit=900000000 --function="claimRewards" --send --proxy=${PROXY} --chain=${CHAIN_ID}
}

calculateRewards() {
    erdpy --verbose contract query ${ADDRESS} --function="calculateRewards" --arguments ${USER_1_HEX} --proxy=${PROXY}
}

getLastCommunityRewardAmountAndEpoch() {
    erdpy --verbose contract query ${ADDRESS} --function="getLastCommunityRewardAmountAndEpoch" --proxy=${PROXY}
}

