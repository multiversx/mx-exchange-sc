# Farm with Community Rewards Smart Contract

## Abstract

This contract is very similar with the regular Farm contract. It is recommended that you go though its README first, as this doc will cover only the differences.

## Introduction

This contract keeps the same computation logic as the standard Farm SC, but in turn, allows the admin to deposit a specific amount of tokens that will be used as rewards for users, instead of minting a specific token. The reward token is different for each farm contract.

## Endpoints

### depositRewards

```rust
    #[payable("*")]
    #[only_admin]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self)
```

This endpoint allows the admin to deposit the reward token, in order for the contract to distribute it to the users.


```rust
    #[only_admin]
    #[endpoint(startProduceCommunityRewards)]
    fn start_produce_community_rewards(
        &self,
        starting_block_offset: Nonce
    )
```

This endpoint allows the admin to start the rewards distribution. It has two particularities from the standard Farm SC. First, it offers the ability to start the distribution at a specific point in time, by providing a __starting_block_offset__ parameter that is added to the __current_block__. The other particularity is that there is a 90 days compulsory period of rewards distribution. In other words, if the rewards available for distribution divided by the number of rewards per block do not cover at least a 90 days period (calculated in number of blocks), then the endpoint would result in a failed transaction.


The other endpoints are the same as in the Farm contract.

## Testing

The same as Farm contract.

## Interaction

The same as Farm contract.

## Deployment

The same as Farm contract.
