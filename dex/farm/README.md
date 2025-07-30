# Farm Smart Contract

## Abstract

Liquidity providers of xExchange are incentivized with MEX rewards in exchange for them locking their LP tokens in Farm contracts.  

## Introduction

This smart contract has the role of generating and distributing MEX tokens to liquidity providers that choose to lock their LP tokens, thus increasing the ecosystem stability.

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
    );
```

The arguments are:

- __reward_token_id__ - MEX token ID
- __farming_token_id__ - token used for farming - LP tokens usually
- __division_safety_constant__ - a constant that is used for math safety functions - increasing precision of reward distribution
- __pair_contract_address__ - almost each farm contract has an associated pair contract, exception being the MEX farm. This address needs to be known because in case of penalty burn, the farm will need the Pair contract in order to convert LP tokens to MEX and then burn them

### enterFarm

```rust
    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(&self);
```

This endpoint receives at least one payment:

- The first payment has to be of type __farming_token_id__. The actual token that is meant to be locked inside the Farm contract.
- The additional payments, if any, will be Farm positions and will be used to be merged with the newly created position, in order to consolidate all previous positions with the current one.

This endpoint will give back to the caller a Farm position as a result. The Farm position is a META esdt that contains, in its attributes, information about the user input tokens and the current state of the contract when the user did enter. This information will be later used when trying to claim rewards or exit farm.

### exitFarm

```rust
    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(&self);
```

This endpoint receives one payment, and that is the Farm Position. Based on an internal counter that the contract keeps track of, which is the __rps__ - meaning reward per share, the contract can calculate the reward that it needs to return to the caller for those specific tokens that he has sent. The output will consist of two payments: the LP tokens initially added and the accumulated rewards.

This contract simulates timestamp-based reward generation by keeping track of the last timestamp that generated rewards and keeps updating on every endpoint execution. Everytime an execution happens, the contract will generate the rewards for the previous time period. This is the case for the first successful transaction within a timestamp interval, so only once per interval this check has to be made and the action to be taken.

If a user decides to exit too early, they will receive a penalty. This contract will take a part of its input LP tokens and will used them to buyback-and-burn MEX. This is done via a smart contract call to the configured pair contract address, via __removeLiquidityAndBuybackAndBurnToken__ endpoint.

### claimRewards

```rust
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self);
```

This endpoint receives at least one payment:

- The first payment is a Farm position that is 'harvested'. So for this position, the contract will calculate the reward and will return it to its caller. The contract will create a new position that has the ```RPS`` (Reward per share) reset.
- The additional payments, if any, will be other Farm positions and will be used to be merged with the newly created one.

### compoundRewards

```rust
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self);
```

This endpoint is similar with claimRewards, the differences being that instead of giving back the rewards to the caller, they are compounded into the newly created position (with the reset RPS). For this to be possible, reward token and farming token have to be the same, hence it is applicable only in case of MEX Farm.

### mergePositions

```rust
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(&self);
```

This endpoint merges two or more farm positions together and returns a single consolidated position to the caller.

## Testing

Aside from the scenario tests, there are a lot of tests that are available in the rust test suite.

## Interaction

The interaction scripts for this contract are located in the dex subdirectory of the root project directory.

## Deployment

The deployment of this contract is done using interaction scripts and it is managed by its admin (regular wallet at the moment, yet soon to be governance smart contract).


# Farm OnBehalf Operations

## Abstract

The Farm OnBehalf operations extend the Farm smart contract with the ability to allow whitelisted contracts to perform actions on behalf of users, enabling enhanced protocol composability while maintaining security through integration with the Permissions Hub.

## Introduction

This module allows third-party contracts to perform farm operations on behalf of users, after being explicitly whitelisted through the Permissions Hub. Users maintain full control over their assets by managing contract permissions, while protocols can build more complex DeFi interactions.

## Endpoints

### enterFarmOnBehalf

```rust
#[payable("*")]
#[endpoint(enterFarmOnBehalf)]
fn enter_farm_on_behalf(&self, user: ManagedAddress) -> EnterFarmResultType<Self::Api>
```

The enterFarmOnBehalf function allows whitelisted contracts to enter farm positions on behalf of users. It receives several arguments:

- __user__ - The address of the user for whom the operation is being performed. This address must have whitelisted the caller contract through the Permissions Hub.
- __payment__ - The tokens to be used are received as payment in the transaction.

The function performs the following steps:
1. Validates that the caller is whitelisted by the user through Permissions Hub
2. Processes the farming tokens payment
3. Claims any pending boosted rewards for the original owner
4. Performs the enter farm operation on behalf of the original owner
5. Sends the new farm token to the caller
6. Sends the collected rewards, if any, to the original owner
7. Updates energy and progress for the original owner

### claimRewardsOnBehalf

```rust
#[payable("*")]
#[endpoint(claimRewardsOnBehalf)]
fn claim_rewards_on_behalf(&self) -> ClaimRewardsResultType<Self::Api>
```

The claimRewardsOnBehalf function enables whitelisted contracts to claim rewards on behalf of the users. This function does not require any address parameter, as the original owner is read from the farm position metadata. The operation requires:

- __payment__ - The farm token must be received as payment in the transaction.

The function performs these steps:
1. Processes the farm token payment
2. Extracts the original owner from the farm token attributes
3. Validates that the caller is whitelisted by the original owner
4. Claims and sends rewards to the original owner
5. Sends the new farm token to the caller

## exitOnBehalf
The exit operation remains under the direct control of the position owner to ensure maximum security. When third-party contracts interact with farming positions through onBehalf operations, they receive and hold the position tokens. These tokens maintain the original owner information in their attributes, protecting the user's ownership rights. To exit their position, users must first reclaim their position tokens from the third-party contract through that protocol's specific mechanisms. Once users have regained control of their position tokens, they can perform the standard exit operation directly through the specific xExchange contract. 
This design ensures users maintain ultimate control over their funds while allowing protocols to build complex DeFi interactions.

## Storage

The contract relies on the Permissions Hub for permission management, thus no additional storage, other than the one holding the Permissions Hub SC address, is required. All whitelisting data is managed through the Permissions Hub contract.


## Deployment

The onBehalf features are part of the core farm contract and require:

1. A deployed Permissions Hub contract
2. Configuration of the Permissions Hub address in the farm contract
3. User whitelisting of contracts that will perform onBehalf operations

