# Proxy DEX Smart Contract

## Abstract

Locked MEX tokens can be used as MEX token in xExchange, with the help of this proxy contract.

## Introduction

This smart contract offers users with Locked MEX the possibility of interacting with the DEX contracts, for operations like Adding Liquidity and Entering Farms, as if they had MEX.

## Endpoints

### init

```rust
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_factory_address: ManagedAddress,
    );
```

### addLiquidityProxy

```rust
    #[payable("*")]
    #[endpoint(addLiquidityProxy)]
    fn add_liquidity_proxy(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    );
```

Add Liquidity Proxy intermediates liquidity adding in a Pair contract as follows:
- The user must send the tokens in the same order as they are in the Pair contract
- The user must configure the slippage as he would in the Pair contract
- The rule is that MEX - and Locked MEX implicitly - is always the second token

The output payment of this contract is not the original LP token and is instead a Wrapped LP token. The reason is that if the user receives directly the LP tokens, he would have had the possibility of removing the liquidity and thus unlocking his Locked MEX.

### removeLiquidityProxy

```rust
    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: BigUint,
        #[payment_nonce] token_nonce: Nonce,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    );
```

Remove Liquidity Proxy intermediates removing liquidity from a Pair contract as follows: the user sends Wrapped LP tokens and receives the First token and the Locked MEX tokens. The address and slippage is configurable as they would be for the Pair contract.

### enterFarmProxy

```rust
    #[payable("*")]
    #[endpoint(enterFarmProxy)]
    fn enter_farm_proxy_endpoint(&self, farm_address: ManagedAddress);
```

Enter Farm Proxy intermediates entering a Farm contract as follows: the user sends Wrapped LP tokens and receives Wrapped Farm tokens. The reasoning for Wrapped Farm tokens is the same as for Wrapped LP tokens.

The following next functions work exactly the same as their analogue functions in the Farm contract, except they take as an input Wrapped Farm tokens, instead of regular Farm tokens.

The original LP tokens and Farm tokens, for which the contract mints Wrapped Tokens, remain in the smart contract. It will use the original tokens for performing the actions on behalf of the user.

### exitFarmProxy

```rust
    #[payable("*")]
    #[endpoint(exitFarmProxy)]
    fn exit_farm_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: Nonce,
        #[payment_amount] amount: BigUint,
        farm_address: &ManagedAddress,
    );
```

### claimRewardsProxy

```rust
    #[payable("*")]
    #[endpoint(claimRewardsProxy)]
    fn claim_rewards_proxy(&self, farm_address: ManagedAddress);
```

### compoundRewardsProxy

```rust
    #[payable("*")]
    #[endpoint(compoundRewardsProxy)]
    fn compound_rewards_proxy(&self, farm_address: ManagedAddress);
```

### mergeWrappedFarmTokens

```rust
    #[payable("*")]
    #[endpoint(mergeWrappedFarmTokens)]
    fn merge_wrapped_farm_tokens(
        &self,
        farm_contract: ManagedAddress
    );
```

This function merges two or more positions of Wrapped Farm (farm positions obtained using Locked MEX instead of MEX and this intermediary contract). In order to merge two positions of this type, the contract uses merge endpoints for the underlying tokens like Farm tokens, Locked MEX tokens, Wrapped LP tokens and so on, and after that, the newly created Wrapped Farm token will just reference the newly created and merged underlying tokens.

### mergeWrappedLpTokens

```rust
    #[payable("*")]
    #[endpoint(mergeWrappedLpTokens)]
    fn merge_wrapped_lp_tokens(&self);
```

This function merges two or more positions of Wrapped LP tokens (LP positions obtained using Locked MEX instead of MEX and this intermediary contract). The same logic as for __mergeWrappedFarmTokens__ is applied.

## Testing

This contract has its own test suite in its subdirectory and it is included in most scenarios that include Locked MEX (Proxy SC, Farm SC with Lock and so on).

## Interaction

The interaction scripts for this contract are located in the dex subdirectory of the root project directory.

## Deployment

The deployment of this contract is done using interaction scripts and it is managed by its admin (regular wallet at the moment, yet soon to be governance smart contract).
