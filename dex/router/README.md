# Router Smart Contract

## Abstract

The Router smart contract is used to easily manage and keep track of existing Pair contracts in a decentralized manner.

## Introduction

This contract allows:

- Deploying

- Upgrading

- Configuring

- Keeping track

of Pair smart contracts deployed as part of xExchange.

## Endpoints

### init

```rust
    #[init]
    fn init(&self, pair_template_address_opt: OptionalValue<ManagedAddress>);
```

The only parameter __pair_template_address_opt__ and it is not mandatory. More about this parameter and how it is used can be found below.

### createNewPair

```rust
    #[endpoint(createPair)]
    fn create_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        initial_liquidity_adder: ManagedAddress,
        opt_fee_percents: OptionalValue<MultiValue2<u64, u64>>,
    );
```

This endpoint is used to create new liquidity pools.

Its arguments are:

- __first_token_id__  - The first token identifier that will represent the liquidity pool.
- __second_token_id__
- __initial_liquidity_adder__ - The address of Price Discovery. In case it isn't used a price discovery mechanism, the argument must be ```Address::zero()```. Alternatively this could be configured as ```OptionalValue<ManagedAddress>```, but for the simplicity of formatting transactions, the zero address was used.
- __opt_fee_percents__ - The fees percents that will be used to configure the newly created pair contract. These are taken into account only in case of the router owner being the caller. Other callers are not allowed to configure these perameters and the default ones will be used.

The way the Router deploys a new Pair smart contract is via ```deploy_from_source_contract``` from the address specified by __pair_template_address__. The way this endpoint works is that it just copies the smart contract bytecode from the source to another instance and it returns the address of the newly created smart contract. The init function is also invoked after the bytecode is copied and before returning.

### upgradePair

```rust
    #[only_owner]
    #[endpoint(upgradePair)]
    fn upgrade_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        total_fee_percent_requested: u64,
        special_fee_percent_requested: u64,
    );
```

UpgradePair works in a similar way as deploy pair. It uses ```upgrade_from_source_contract``` and it works exactly the same as ```deploy_from_source_contract```, with the distinction that the destination contract has to already be deployed in order to be upgraded from source contract.

### issueLpToken

```rust
    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        #[payment_amount] issue_cost: BigUint,
        pair_address: ManagedAddress,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    );
```

In order to simplify the issuing of LP tokens and their management, the Router smart contract is the owner and manager of the LP tokens. The way it works is that the router issues the tokens and then it sets the roles of mint and burn to the pair contracts.

## Testing

There are four test suites around this contract:

- __scenario__ tests are located in the _dex/scenarios_ directory. The tests can be ran using __mandos-test__
- __rust__ tests are written using __rust_testing_framework__. The tests can be ran as any other rust test using __cargo-test__. This test suite is to be preferred and will be extended and maintained over the scenario tests because the testing framework offers programmatic testing.
- __legolas__ tests are python scripts that use actual live transactions on testing setups

## Interaction

The interaction scripts are located in the _dex/interaction_ directory. They are written in python and mxpy is required in order to be used. Interaction scripts are scripts that ease the interaction with the deployed contract by wrapping mxpy sdk functionality in bash scripts. Make sure to update the PEM path and the PROXY and CHAINID values in order to correctly use them.

## Deployment

The deployment of this contract is done using mxpy, interaction scripts, or any other tool, in a standalone manner or by previously deploying the template pair smart contract.
