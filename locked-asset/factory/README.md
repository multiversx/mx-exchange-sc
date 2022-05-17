# Locked Asset Factory Smart Contract

## Abstract

This smart contract is the 'father' of all Locked MEX tokens as any contract that needs those tokens, must request them here.

## Introduction

This smart contract has the role of creating Locked MEX tokens. The rule is that only one contract per shard can hold the 'create' role for a META esdt, and because many contracts needed to mint/create Locked MEX, this factory was created so it can serve these requests from only one place.

## Endpoints

### init

```rust
    #[init]
    fn init(
        &self,
        asset_token_id: TokenIdentifier,
        default_unlock_period: MultiValueEncoded<UnlockMilestone>,
    );
```

The arguments are:

- __asset_token_id__ - MEX token ID
- __default_unlock_period__ - each Locked MEX has an unlock schedule that is an array of unlock milestones. An unlock milestone is a combination of Unlock Epoch + Unlock Percent. An unlock schedule is created usually using a starting epoch combined with an unlock period.

### createAndForward

```rust
    #[endpoint(createAndForward)]
    fn create_and_forward(
        &self,
        amount: BigUint,
        address: ManagedAddress,
        start_epoch: Epoch,
    )
```

This is the endpoint called by other smart contracts when in need of creating Locked MEX. The contract will use the arguments and will create and send the __amount__ of Locked MEX to __address__ having the unlock schedule starting at __start_epoch__ with the __default_unlock_period__.

### unlockAssets

```rust
    #[payable("*")]
    #[endpoint(unlockAssets)]
    fn unlock_assets(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: BigUint,
        #[payment_nonce] token_nonce: Nonce,
    );
```

This endpoint receives Locked MEX as payment and returns, if possible (if at least one Unlock Milestone was reached), regular Fungible MEX and the leftover Locked MEX, with updated Unlock Milestones.

### lockAssets

```rust
    #[payable("*")]
    #[endpoint(lockAssets)]
    fn lock_assets(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_amount] payment_amount: BigUint,
    )
```

This endpoint receives MEX tokens as payment and returns Locked MEX tokens. The reason why someone would do this is because Locked MEX is used in different mechanisms where MEX cannot be used. Furthermore, Locking is beneficial to the Ecosystem.

### mergeLockedAssetTokens

```rust
    #[payable("*")]
    #[endpoint(mergeLockedAssetTokens)]
    fn merge_locked_asset_tokens(&self)
```

The endpoint merges two or more Locked MEX tokens with different nonces together. It handles reconstructing the Unlock Schedule in a fair manner and with high precision.

## Testing

This contract has its own test suite in its subdirectory and it is included in most scenarios that include Locked MEX (Proxy SC, Farm SC with Lock and so on).

## Interaction

The interaction scripts for this contract are located in the dex subdirectory of the root project directory.

## Deployment

The deployment of this contract is done using interaction scripts and it is managed by its admin (regular wallet at the moment, yet soon to be governance smart contract).
