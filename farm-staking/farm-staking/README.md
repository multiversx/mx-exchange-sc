# Farm Staking Contract

## Abstract

As the ecosystem grows, the necessity for new means of generating staking rewards without the added inflation appears. So the farm staking contract was needed, where all the rewards are being distributed from already minted tokens. Furthermore, the contract works with different types of tokens, not only LP ones.

## Introduction

This contract allows users to stake their tokens and/or LP tokens and earn rewards. It works in conjunction with the Farm Staking Proxy contract and offers the configuration means for each farm, from rewards handling, to merging tokens and periods setup.

## Setup Endpoints

The setup workflow for the farm staking contract is as follows:
- The SC deployment
- Setting up the farming token. Here it is important to register/issue the farm token and to also set its local roles.
- Setting up the reward computation rate. The rewards are not minted, but instead they are distributed from a predefined amount that is set by the owner. Also, as time passes, the reward pool can be replenished through the ```topUpRewards```  endpoint. Also, the SC allows the owner to set a maximum APR percentage, that limits the amount of generated rewards, in order to avoid competitions.
- Setting the unbond period for the staked amounts.

### init

```rust
    #[init]
    fn init(
        &self,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        max_apr: BigUint,
        min_unbond_epochs: u64,
    );
```

The init function is called when deploying/upgrading the smart contract. It receives the following arguments:

- __farming_token_id__ - the farming token ID, which will also be the reward token. It is saved in the storage and cannot be changed after deployment.
- __division_safety_constant__ - a constant used for precise calculations. The recommended format is 10^9. It is saved in the storage and cannot be changed after deployment, unless the contract is upgraded.
- __max_apr__ - a percentage of max APR, which will limit the user's rewards, with two decimals precision (i.e. 10_000 = 100%). Can be more than 100% and can be updated as time passes.
- __min_unbond_epochs__ - Number of epochs the user has to wait between unstake and unbond. It can be updated as time passes.

### topUpRewards

```rust
    #[payable("*")]
    #[endpoint(topUpRewards)]
    fn top_up_rewards(&self);
```

Because rewards are not minted but are instead distributed from a predefined amount, as time passes the rewards will be depleted. So new tokens must be added to the rewards pool, through this endpoint, to be further distributed to stakers.

### setPerBlockRewardAmount

```rust
    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(
        &self, 
        per_block_amount: BigUint
    );
```
Endpoint that sets the amount of reward tokens that are distributed per block. Takes as an argument the amount __per_block_amount__.

### setMaxApr

```rust
    #[endpoint(setMaxApr)]
    fn set_max_apr(
        &self, 
        max_apr: BigUint
    );
```

Endpoint that sets the maximum amount of reward tokens that can be generated, expressed as a percentage. It acts like a capping value for the reward calculation. Takes as an argument the maximum APR percentage __max_apr__.

### setMinUnbondEpochs

```rust
    #[endpoint(setMinUnbondEpochs)]
    fn set_min_unbond_epochs(
        &self, 
        min_unbond_epochs: u64
    ); 
```

Endpoint that sets the number of unbonding epochs for the staked amounts. Each staking farm has an unboding period, that must pass before an user can withdraw his/hers staked amounts. Takes as an argument the number of unbonding epochs __min_unbond_epochs__.

### addAddressToWhitelist

```rust
    #[endpoint(addAddressToWhitelist)]
    fn add_address_to_whitelist(
        &self, 
        address: ManagedAddress
    ); 
```

Endpoint that sets an address as whitelisted, in order for it to be able to do different operations like staking or claiming rewards. At this moment, the whitelisted requirement is imposed only in the proxy related endpoints, in order to have a reliable data feed for the farm staking logic.
The whitelisting of an address is stored as a bool variable for each address.

### removeAddressFromWhitelist

```rust
    #[endpoint(removeAddressFromWhitelist)]
    fn remove_address_from_whitelist(
        &self, 
        address: ManagedAddress
    ); 
```

Endpoint that removes an address from being whitelisted. It takes as an argument the __address__ that is to be removed.

### startProduceRewards

```rust
    #[endpoint(startProduceRewards)]
    fn start_produce_rewards(&self);
```
Endpoint that starts the rewards distribution.

 endProduceRewards

```rust
    #[endpoint]
    fn end_produce_rewards(&self);
```

Endpoint that pause or ends the rewards distribution.

## Public endpoints

### stakeFarm

```rust
    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(&self);
```

Endpoint that allows an user to stake his tokens in order to enter to enter the staking farm. It receives the farming_token as a payment and it sends the farm_token back to the caller.

### stakeFarmThroughProxy

```rust
    #[payable("*")]
    #[endpoint(stakeFarmThroughProxy)]
    fn stake_farm_through_proxy(
        &self,
        staked_token_amount: BigUint,
    );
```

A special endpoint designed to work together with the proxy farm staking contract. It can only be called by whitelisted addresses (in our case the proxy contract) and receives the __staked_token_amount__  as a variable instead of a real transfer. Is is payable, so it allows the caller to also send his current position.
The way the endpoint works is that when calling the base staking function, it creates a new composed set of payments, consisting of the simulated transfer and the actual transfers. The contract then calculates the farm_token amount and sends it to the proxy contract, that will then calculate the corresponding dual_yield tokens, which are then send to the actual user. The farm_tokens will remain inside the proxy contract.

### mergeFarmTokens

```rust
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(&self);
```

A payable endpoint that allows the user to merge his farm staking tokens. It is also the method that is called by the proxy farm staking contract in order to give the user his combined position. In this case, a NFT MultiTransfer is being send by the proxy contract with the lp_farm token and the proxy_dual_yield token. The new lp_farm tokens are being minted and sent back to the proxy contract, which will then send the new dual_yield tokens to the user. 

### unstakeFarm

```rust
    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(&self);
```

Endpoint that allows the user to unstake his farm tokens. It receives the farm_token as a payment and it sends the unbond_farming_token back to the caller. The farm_tokens are burnt and the unbond_farming_tokens are then minted through the ``nft_create_tokens`` function, which encodes the ``UnbondSftAttributes`` in the newly created tokens. In the end, the calculated rewards are sent to the caller.

### unstakeFarmThroughProxy

```rust
    #[payable("*")]
    #[endpoint(unstakeFarmThroughProxy)]
    fn unstake_farm_through_proxy(&self);
```

The special endpoint that allows the unstaking of the dual_yield token. It can only be called by whitelisted addresses (in our case the proxy contract).
The way the endpoint works is that it receives an exact amount of two payments, that have to be in a specific order. The first payment consists of staking tokens, that are taken from the liquidity pool and that will be sent to the user on unbond. The second payment consists of farm tokens that follow the general ``unstakeFarm`` workflow explained above.

### unbondFarm

```rust
    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(&self);
```

Endpoint that allows the user to unbond his farming tokens. As previously stated, the ``unstakeFarm`` endpoint gives the user unbond_farming_tokens, that have the unbonding period encoded. The unbond function receives the unbond_farming_tokens as a payment and decodes the unbonding period in order to check if the tokens can be unbonded. If the unbonding period has passed, the unbond_farming_tokens are burnt and then the farming_tokens are sent back to the caller.

### claimRewards

```rust
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self);
```

Endpoint that allows the caller to send his farm tokens and to receive the corresponding rewards. The sent farm tokens are burnt and new tokens are minted, in order to reset that user's position.

### claimRewardsWithNewValue

```rust
    #[payable("*")]
    #[endpoint(claimRewardsWithNewValue)]
    fn claim_rewards_with_new_value(
        &self,
        new_farming_amount: BigUint,
    );
```

As stated in the proxy contract documentation, the staking farm tokens are also eligible for rewards. But because during the period between when the rewards generated for the lp_farm tokens were claimed and the claiming of the rewards generated by the staking farm token, the lp ratio may differ, a specific function is needed to get a new quote from the LP contract. The endpoint then calls the generic claim_rewards method, that burns the farm tokens and mints new ones and then sends the rewards based on the new quote.

### compoundRewards

```rust
    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self);
```

Payable endpoint that allows the caller to harvest the rewards generated by the staking farm and reinvest them seamlessly, within a single endpoint. It burns the current farm tokens and computes the actual position with the rewards included.
