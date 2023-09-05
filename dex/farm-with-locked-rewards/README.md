# Farm With Locked Rewards Smart Contract

## Abstract

Liquidity providers of xExchange are incentivized with locked rewards in exchange for them locking their tokens in Farm contracts.  

## Introduction

This smart contract has the role of generating and distributing locked reward tokens to users that choose to enter the farm contract (both LP or single token farms). This contract relies heavily on the standard Farm contract, the difference being that the generated rewards are locked.

## Endpoints

### init

```rust
    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    );
```

The arguments are:

- __reward_token_id__ - MEX token ID
- __farming_token_id__ - token used for farming - could be LP or single token
- __division_safety_constant__ - a constant that is used for math safety functions - increasing precision of reward distribution
- __pair_contract_address__ - almost each farm contract has an associated pair contract, exception being the MEX farm. This address needs to be known because in case of penalty burn, the farm will need the Pair contract in order to convert LP tokens to MEX and then burn them
- __owner__ - the owner of the contract
- __admins__ - a list of addresses that are allowed to execute a set of actions

### enterFarm

```rust
    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(&self);
```

This endpoint receives at least one payment:

- The first payment has to be of type __farming_token_id__. The actual token that is meant to be locked inside the Farm contract.
- The additional payments, if any, will be Farm positions and will be used to be merged with the newly created position, in order to consolidate all previous positions with the current one.

This endpoint will give back to the caller a Farm position as a result. The Farm position is a META esdt that contains, in its attributes, information about the user input tokens and the current state of the contract when the user did enter. This information will be later used when trying to claim rewards or exit farm.

### exitFarm

```rust
    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm_endpoint(&self);
```

This endpoint receives one payment, and that is the Farm Position. Based on an internal counter that the contract keeps track of, which is the __rps__ - meaning reward per share, the contract can calculate the reward that it needs to return to the caller for those specific tokens that he has sent. The output will consist of two payments: the LP tokens initially added and the accumulated rewards.

This contract simulates per-block-reward-generation by keeping track of the last block that generated mex and keeps updating on every endpoint execution. Everytime an execution happens, the contract will generate the rewards for previous blocks. This is the case for the first successful TX inside a block, so only once per block this check has to be made and the action to be taken.

### claimRewards

```rust
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(&self);
```

This endpoint receives at least one payment:

- The first payment is a Farm position that is 'harvested'. So for this position, the contract will calculate the reward and will return it to its caller. The contract will create a new position that has the ```RPS`` (Reward per share) reset.
- The additional payments, if any, will be other Farm positions and will be used to be merged with the newly created one.

### compoundRewards

```rust
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards_endpoint(&self);
```

This endpoint is similar with claimRewards, the differences being that instead of giving back the rewards to the caller, they are compounded into the newly created position (with the reset RPS). For this to be possible, reward token and farming token have to be the same, hence it is applicable only in case of single token Farm.

### mergePositions

```rust
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self);
```

This endpoint merges two or more farm positions together and returns a single consolidated position to the caller.

## Testing

The testing for this contract has been done through Rust Testing Framework.

## Interaction

The interaction scripts for this contract are located in the dex subdirectory of the root project directory.

## Deployment

The deployment of this contract is done using interaction scripts and it is managed by its admin.
