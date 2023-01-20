# LKMEX transfer Smart Contract

## Abstract

Locked MEX is an untransferable token outside this contract.

## Introduction

This smart contract has the role of allowing the transfer of Locked MEX to another account by locking the funds inside it for a period of time. The use of this contract is limited transfers not allowed more than once for a period of time.

## Endpoints

### init

```rust
    #[init]
    fn init(
        &self,
        token: TokenIdentifier,
        unlock_transfer_time: Epoch,
        epochs_cooldown_duration: Epoch,
    ) 
```

The arguments are:

- __token__ - Locked MEX token ID
- __unlock_transfer_time__ - time in epochs after which the locked assets will become available for claiming
- __epochs_cooldown_duration__ - time in epochs that limit an address from using the contract again for locking tokens

### lockFunds

```rust
    #[payable("*")]
    #[endpoint(lockFunds)]
    fn lock_funds(&self, address: ManagedAddress);
```

The arguments are:

- __address__ - the address that allowed to claim the funds

### withdraw

```rust
    #[endpoint(withdraw)]
    fn withdraw(&self);
```

This endpoint will throw an error if the caller has nothing to claim or if the lock time of the funds is still up.

## Testing

Aside from the scenario tests, there are a lot of tests that are available in the rust test suite.

## Interaction

The interaction scripts for this contract are located in the dex subdirectory of the root project directory.

## Deployment

The deployment of this contract is done using interaction scripts and it is managed by its admin (regular wallet at the moment, yet soon to be governance smart contract).
