# MEX Governance SC

## Introduction

This contract is meant to give MEX holders the oportunity to propose and vote actions, in a decentralized manner.

## Public Endpoints

### Init

The init function often offers a lot of information about what the contract does and what are its settings, so let's see what argument the init function receives:

```rust
fn init(
        &self,
        quorum: BigUint,
        voting_delay_in_blocks: u64,
        voting_period_in_blocks: u64,
        vote_nft_id: TokenIdentifier,
        mex_token_id: TokenIdentifier,
        min_weight_for_proposal: BigUint,
        governance_token_ids: ManagedVec<TokenIdentifier>,
        price_providers: MultiValueEncoded<MultiValue2<TokenIdentifier, ManagedAddress>>
    )
```

- quorum -> The difference between the upvotes and downvotes for a proposal to be considered successful.
- voting_delay_in_blocks -> Once a proposal is created, users cannot vote immediately, to avoid impulsivity. A number of blocks must be passed before voting is enabled.
- voting_period_in_blocks -> Self explanatory.
- vote_nft_id -> Each vote will be represented by an NFT. We'll see later what attributes this NFT has.
- mex_token_id -> Self explanatory.
- min_weight_for_proposal -> The minimum weight for creating a proposal. We'll see later what this weight means.
- governance_token_ids -> A list of tokens that users can vote with. Will contain MEX, but also LP_MEX, FARM_MEX and so on.
- price_providers -> A list of pairs, Token-Address, where the contract can query a token's weight.

### Propose

The propose functions is used to create a proposal. This function receives as input an argumet of type ProposalCreationArgs, which tells what a proposal is.

```rust
pub struct ProposalCreationArgs {
    pub description: ManagedBuffer,
    pub actions: ManagedVec<Action>,
}
```

A proposal consists of a description and a set of actions.

An action is a Smart Contract call, as described above.

```rust
pub struct Action {
    pub gas_limit: u64,
    pub dest_address: ManagedAddress,
    pub payments: ManagedVec<ManagedBuffer>,
    pub endpoint_name: ManagedBuffer,
    pub arguments: ManagedVec<ManagedBuffer>,
}
```

Making a proposal requires transferring tokens. So make sure you either use ESDTTransfer, ESDTNFTTransfer of a similar function to call this endpoint.
Also, make sure the min_weight_for_proposal requiremet is passed, etherwhise the tx will fail.
On success, the function will return the id of the newly created proposal.

The caller will receive an NFT that will represent his vote (upvote is this case). The tokens used for voting will remain locked in the SC until the proposal ends (with either success or fail). The vote NFT will be sent back to the SC in order to redeem the tokens that were used for making the proposal. Same applies to basic voting too.

### Upvote

This function is used to positive-vote (upvote) a proposal It receives the proposal id as an argument. It is callable with as an ESDT transfer-and-execute function.

### Downvote

This function is used to negative-vote (downvote) a proposal It receives the proposal id as an argument. It is callable with as an ESDT transfer-and-execute function.

### Execute

This function is used to execute a proposal. It receives the proposal id as an argument. Callable by anyone. Can be used only for successful proposals and only once per proposal.

### Redeem

This function is used to redeem the tokens that were used for voting purpose. It is callable with as an ESDT-NFT transfer-and-execute function. The NFT accepted will be the Vote NFT.

## Technical Details

### Vote NFT

The Vote NFT is used as an external distributed storage.

```rust
pub struct VoteNFTAttributes {
    pub proposal_id: u64,
    pub vote_type: VoteType,
    pub vote_weight: BigUint,
    pub voter: ManagedAddress,
    pub payment: EsdtTokenPayment,
}
```

The most important fields here are: ```proposal_id```, which needs to be checked when redeem is called, to make sure the proposal ended, and the ```payment```, which is what the caller will get back when trying to redeem the tokens for a vote to an ended proposal.

### Vote Weight

As described above, multiple tokens can be used to propose/vote. The value of each vote (weight) must be denominated in the amount of MEX tokens that a certain tokens holds behind it. The contract knows to use exec on dest ```updateAndGetTokensForGivenPositionWithSafePrice``` for each token's price provider in order to get the amount.
