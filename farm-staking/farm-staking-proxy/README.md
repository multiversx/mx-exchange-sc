# Farm Staking Proxy Contract

## Abstract

Metastaking is a new way to empower users that provide liquidity, by giving them the ability to stake their tokens in order to receive rewards as stakers. But how can users stake assets that they currently do not hold? Well, through trustworthy simulated transfers. By using a proxy contract, that is whitelisted in the staking contract, we can send the data feed to the staking contract, like the simulated transfers. By doing so, the proxy contract keeps the actual farm token amounts, and the staking process is organized through a dual yield token, that is minted and burned by this SC. 

## Introduction

This SC works in conjunction with the Farm Staking contract and offers the configuration means for the dual yield token, that takes care of the staking logic of the farm staking process. As a high level overview, we can underline the following steps:
- The user follows the usual steps to enter a simple farm: add liquidity + enter farm with the LP tokens
- He then sends the farming tokens to the farm staking proxy contract
- The proxy contract calculates the user's position and simulates a transfer on his behalf to the staking contract. By being whitelisted as a trustworthy address, the staking contract accepts the data as a simulated transfer
- The staking contract calculates the farming token (by quoting the LP contract) and sends the farm staking position to the proxy contract
- The proxy contract keeps the farming token and sends the dual yield token instead to the user
- The user can then use the dual yield token to claim his rewards or unstake his position

To sum it up, we can say that the two contract farm staking mechanism allows the user to do all the usual operations for the normal farming contract, not directly, but through a proxy contract. And he can do all this by using the dual yield token.

## Setup Endpoints

### init

```rust
    #[init]
    fn init(
        &self,
        lp_farm_address: ManagedAddress,
        staking_farm_address: ManagedAddress,
        pair_address: ManagedAddress,
        staking_token_id: TokenIdentifier,
        lp_farm_token_id: TokenIdentifier,
        staking_farm_token_id: TokenIdentifier,
        lp_token_id: TokenIdentifier,
    );
```
The deployment function, it receives the required variables like token ids and addresses, in order to properly setup the proxy contract.

### registerDualYieldToken

```rust
    #[payable("EGLD")]
    #[endpoint(registerDualYieldToken)]
    fn register_dual_yield_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    );
```

Endpoint that allows the setup of the dual yield token. It receives the standard parameters for creating a new token, as is payable in eGLD.

### setLocalRolesDualYieldToken

```rust
    #[endpoint(setLocalRolesDualYieldToken)]
    fn set_local_roles_dual_yield_token(&self);
```

Endpoint that allows the setup of the dual yield token roles. It adds the following roles: NftCreate, NftAddQuantity, NftBurn.

## Public Endpoints

### stakeFarmTokens

```rust
    #[payable("*")]
    #[endpoint(stakeFarmTokens)]
    fn stake_farm_tokens(&self);
```

The first endpoint in the farm staking workflow. It receives the farming token as a single or as a multiple payment. The endpoint calculates the position for each payment and burns the current dual yield token for the corresponding nonce, if there is any. The workflow continues by quoting the LP contract of the correct token amount and then simulates a token transfer with that amount towards the farm staking contract. It will then receive the corresponding farm staking token amount (amount that will remain inside the contract) and will send the user the corresponding dual yield token.
It is important to mention that only the proxy contract can simulate the token transfer, by being whitelisted inside the farm staking contract to do so. That means that any outside attempts to replicate this process will fail in the staking contract.
Another aspect that is worth mentioning is that the endpoint will try to merge the user's position. For that, it calls the merging function of the farm staking contract in order to give the user a combined position.

### claimDualYield

```rust
    #[payable("*")]
    #[endpoint(claimDualYield)]
    fn claim_dual_yield(&self);
```

For claiming rewards from the farm staking contract, the user has to send his dual yield tokens to the proxy contract as a payment. Based on this payment, the proxy contract identifies the corresponding position for the user and burns those dual yield tokens. It then uses the staking farm tokens to claim the corresponding rewards. In the end, the proxy contract sends those claimed rewards to the user, along with a new, reset position for the dual yield tokens.
One thing to note here is that between claiming rewards in the farming contract and the staking contract, the balance of the LP token may vary. Because of that, the proxy contract first harvest the rewards from the farming contract with the initial known value and then requotes the LP contract to get the new LP ratio (that may or may not vary). It then harvest rewards with the new value.

### unstakeFarmTokens

```rust
    #[payable("*")]
    #[endpoint(unstakeFarmTokens)]
    fn unstake_farm_tokens(
        &self,
        pair_first_token_min_amount: BigUint,
        pair_second_token_min_amount: BigUint,
    );
```

To unstake his current position, a user must send the desired amount of dual yield tokens to the proxy contract. At this moment, the proxy contract knows, based on the sent dual yield token, both the farm token position and staking token position. The first step is for the proxy contract to withdraw the LP tokens from the farms and the liquidity from the pair contract. After that all the harvested rewards, the resulting eGLD from removing the LP token and the unstake position of the staking token are all sent to the user. The unstaking process is ended with the burning of the dual yield tokens.
It is important to note that because of the user’s unstaked position, an unbonding period is not needed.
