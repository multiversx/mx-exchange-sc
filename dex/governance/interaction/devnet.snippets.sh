WALLET_PEM="/home/elrond/elrondsdk/erdpy-venv/lib/python3.8/site-packages/erdpy/testnet/wallets/users/mike.pem"
PROXY="https://devnet-gateway.multiversx.com"
CHAIN_ID="D"

GOVERNANCE_WASM_PATH="/home/elrond/Github/sc-dex-rs/dex/governance/output/governance.wasm"
ESDT_ISSUE_ADDRESS="erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u"

VOTE_NFT_NAME="0x564f5445"
VOTE_NFT_TICKER="0x564f5445"

issueToken() {
    erdpy --verbose contract call ${ESDT_ISSUE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="issue" \
        --arguments 0x4d4558 0x4d4558 0xfffffffffffffffffffffffffffffffffffff 18 \
        --send || return
}

MEX_TOKEN_ID="0x4d45582d373631343430"

issueVoteNFT() {
    erdpy --verbose contract call ${ESDT_ISSUE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --value=50000000000000000 \
        --function="issueNonFungible" \
        --arguments ${VOTE_NFT_NAME} ${VOTE_NFT_TICKER} \
        --send || return
}

VOTE_NFT_ID="0x564f54452d383064663163"

QUORUM=100
VOTING_DELAY_IN_BLOCKS=10
VOTING_PERIOD_IN_BLOCKS=60
MIN_WEIGHT_FOR_PROPOSAL=100

deployGovernanceSC() {
    erdpy --verbose contract deploy --recall-nonce \
        --bytecode=${GOVERNANCE_WASM_PATH} \
        --pem=${WALLET_PEM} \
        --gas-limit=200000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --arguments ${QUORUM} \
        ${VOTING_DELAY_IN_BLOCKS} \
        ${VOTING_PERIOD_IN_BLOCKS} \
        ${VOTE_NFT_ID} \
        ${MEX_TOKEN_ID} \
        ${MIN_WEIGHT_FOR_PROPOSAL} \
        0x0000000a4d45582d373631343430 \
        --send || return
}

CONTRACT_ADDRESS="erd1qqqqqqqqqqqqqpgqcqm9vxzc3ghjgxfyd5dps6xhwpuhgsg6a4sqtsqt6n"
CONTRACT_ADDRESS_HEX="0x00000000000000000500c0365618588a2f2419246d1a1868d7707974411aed60"

setSpecialRolesVoteNFT() {
    erdpy --verbose contract call ${ESDT_ISSUE_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="setSpecialRole" \
        --arguments ${VOTE_NFT_ID} ${CONTRACT_ADDRESS_HEX} 0x45534454526f6c654e4654437265617465 0x45534454526f6c654e46544275726e \
        --send || return
}

PROPOSE_TRANSFER_AMOUNT=0xFFFF
PROPOSE_ARGS=0x000000010100000000

propose() {
    erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${MEX_TOKEN_ID} ${PROPOSE_TRANSFER_AMOUNT} 0x70726f706f7365 ${PROPOSE_ARGS} \
        --send || return
}

propose() {
    erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${MEX_TOKEN_ID} ${PROPOSE_TRANSFER_AMOUNT} 0x70726f706f7365 ${PROPOSE_ARGS} \
        --send || return
}

getPropose() {
    erdpy --verbose contract query ${CONTRACT_ADDRESS} \
        --proxy=${PROXY} \
        --function="getProposal" \
        --arguments $1
}

getProposalStatus() {
    erdpy --verbose contract query ${CONTRACT_ADDRESS} \
        --proxy=${PROXY} \
        --function="getProposalStatus" \
        --arguments $1
}

getQuorum() {
    erdpy --verbose contract query ${CONTRACT_ADDRESS} \
        --proxy=${PROXY} \
        --function="getQuorum"
}

upvote() {
    erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${MEX_TOKEN_ID} ${PROPOSE_TRANSFER_AMOUNT} 0x7570766f7465 $1 \
        --send || return
}

downvote() {
    erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTTransfer" \
        --arguments ${MEX_TOKEN_ID} ${PROPOSE_TRANSFER_AMOUNT} 0x646f776e766f7465 $1 \
        --send || return
}

execute() {
    erdpy --verbose contract call ${CONTRACT_ADDRESS} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="execute" \
        --arguments $1 \
        --send || return
}

SELF_ADDR="erd1uv40ahysflse896x4ktnh6ecx43u7cmy9wnxnvcyp7deg299a4sq6vaywa"

redeem() {
    erdpy --verbose contract call ${SELF_ADDR} --recall-nonce \
        --pem=${WALLET_PEM} \
        --gas-limit=60000000 \
        --proxy=${PROXY} --chain=${CHAIN_ID} \
        --function="ESDTNFTTransfer" \
        --arguments ${VOTE_NFT_ID} $1 0x01 ${CONTRACT_ADDRESS_HEX} 0x72656465656d \
        --send || return
}
