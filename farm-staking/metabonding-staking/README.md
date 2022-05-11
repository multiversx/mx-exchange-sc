# Metabonding Staking Contract

## Abstract

Metabonding is a new community bootstrapping product, that allows users to receive tokens from projects enrolled in the Metabonding program, based on their staked eGLD and locked tokens.

## Introduction

The Metabonding staking contract works in conjunction with the main Metabonding contract.
The workflow can be summarized like so:
- Projects apply to the Metabonding program, by allocating a fixed number of tokens that will be distributed to users based on weekly snapshots.
- The user can stake his locked assets, along with his staked eGLD that is included by default in the snapshots, in order to get a portion of the allocated tokens.
- Daily snapshots are taken, with weekly reward distribution based on the average staked balance.

The Metabonding staking contract takes care of the staking part of the workflow, by storing the staked locked token amounts.

## Endpoints

### stakeLockedAsset

```rust
    #[payable("*")]
    #[endpoint(stakeLockedAsset)]
    fn stake_locked_asset(&self);
```

Payable endpoint that allows the user to stake his locked assets. If the user already has a staking position, the tokens are merged through the locked asset factory contract. An user entry is stored, containing the information as shown below, along with the total locked asset staked supply.

```rust
pub struct UserEntry<M: ManagedTypeApi> {
    pub token_nonce: u64,
    pub stake_amount: BigUint<M>,
    pub unstake_amount: BigUint<M>,
    pub unbond_epoch: u64,
}
```

### unstake

```rust
    #[endpoint]
    fn unstake(
        &self, 
        amount: BigUint
    );
```

Endpoint that allows the user to specify how many locked assets he wants to unstake. When calling the endpoint, the user does not actually receive the tokens back, but he sort of states the intention to take back the tokens. Based on the current unbond duration, the user storage entry is updated with the amount he wants to unstake and the unbonding duration. In case of a second unstake, if the unbonding period is not finished, the unbonding counter is reset.

### unbond

```rust
    #[endpoint]
    fn unbond(&self);
```

Endpoint the allows the user to receive his tokens, considering the unbonding period is over. He receives the amount that he previously unstaked, with the corresponding token nonce.
