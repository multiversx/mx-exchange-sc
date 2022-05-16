# Pair Smart Contract

## Abstract

The Pair smart contract is used to allow the users to exchange tokens in a decentralized manner.

## Introduction

This contract allows users to provide liquidity and to swap tokens. Users are incentivized to add liquidity by earning rewards from fees and by being able to enter farms, thus earning even more rewards. This contract is usually deployed by the router smart contract and it (usually) has no dependency, as it is used as a DeFi primitive.

## Endpoints

### init

```rust
    #[init]
    fn init(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        router_address: ManagedAddress,
        router_owner_address: ManagedAddress,
        total_fee_percent: u64,
        special_fee_percent: u64,
        initial_liquidity_adder: OptionalValue<ManagedAddress>,
    );
```

The init function is called when deploying/upgrading a smart contract. It receives several arguments which the SC cannot function without:

- __first_token_id__ - A Pair smart contract can be for example WEGLD-MEX pair. In this case WEGLD is the first token id, while MEX is the second token id. Their order matters, and throughout the other functions and the code, a clear distinction between them must be made, as they cannot be used interchangeably.
- __second_token_id__
- __router_address__ - The address of the router smart contract. In most cases it is the caller address. The router address, together with the router owner address act as managers. They have the rights to upgrade and modify the contract's settings until a multisign dApp implementation will be ready and in place.
- __router_owner_address__
- __total_fee_percent__ - Each time a swap is made, a fee is charged. The total percent is configurable by this parameter, but is usually 300, representing a percentage of 0.3, base points being 1_000.
- __special_fee_percent__ - The total fee is handled in two different manners. By default, the fee is reinvested into the pool, thus increasing the amount of tokens of liquidity providers. Part of the total fee is handled differently. More specific, a value of 50, representing a percentage of 0.05 of each swap is used to buyback and burn MEX tokens. The special fee can also be split in many equal parts and can be configured to be sent to other addresses also. The convention is that, if a configured special fee destination address is zero, then the tokens should be burned (the way it is configured in Maiar Exchange as well).
- __initial_liquidity_adder__ - An optional argument that is usually a Price Discovery smart contract. Used to decrease changes and impact of Pump & Dump at new listings.

### addLiquidity

```rust
    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    );
```

Adding liquidity is a one TX process. Initially it was designed to be a 3 TX process consisting in: sending the first token, sending the second token, calling the actual add endpoint. Now that MultiTransfer function is available, the process was simplified by sending both tokens and calling the add liquidity endpoint all at once.

Adding of liquidity never changes the ratio between the two tokens. Considering __rA__ and __rB__ are reserves of the first token and reserves of the second token and __aA__ and __aB__ are the amounts of first token and second token that are desired to be added as liquidity, the following formula has to be true:

```rA / rB = aA / aB``` stating that the ratio of the tokens added to the liquidity pool has to be the same as the tokens that are already in the pool.

Calculating them is easy, since one of the desired values to be added __aA__ and __aB__ can be fixated and the other one comes from the formula above.

First add of liquidity sets the ratio and hence the price of the tokens, because the first add goes through without having to respect any formula, since no tokens are in the pool, so there's no ratio to be taken into account.

As stated above, this function receives two payments, the first token payment and the second token payment. The order matters, so the two payments have to be in the same order as the tokens are in the contract. The arguments of this function are:

- __first_token_amount_min__ - The minimum amounts throught all the endpoints in the contract are used to set the slippage. The way it works is the following: when the above formula is applied and the resulted __aB__ is bigger than the transferred __aB__, the transferred __aB__ will be fixated and the __aA__ will be calculated using the formula. The resulted __aA__ has to be between the transferred __aA__ and the __first_token_amount_min__, thus setting the accepted range/slippage.
- __second_token_amount_min__

### addInitialLiquidity

```rust
    #[payable("*")]
    #[endpoint(addInitialLiquidity)]
    fn add_initial_liquidity(&self);
```

This endpoint is ment to be called by Price Discovery smart contract. The contract works in the following way:

1. If __initial_liquidity_adder__ is configured to be ```Some(Address)```, then this address is ment to be the Price Discovery smart contract, and the pair will expect it to call this endpoint. The endpoint is just callable by the configured address and the __add_liquidity__ endpoint is only callabe by anyone after the initial liquidity was added. This mechansim was introduced for reducing Pump & Dump activity at new listings.
2. If __initial_liquidity_adder__ is configured to be ```None```, then this endpoint is not callable, and the public __add_liquidity__ endpoint should be used instead.

### removeLiquidity

```rust
    #[payable("*")]
    #[endpoint(removeLiquidity)]
    fn remove_liquidity(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] liquidity: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    );
```

Removing liquidity is also a one TX flow. Removing liquidity happens as a liquidity provider sends his LP tokens back to the Pair smart contract and provides the parameters that were discussed at the add liquidity flow above - and receives back both types of tokens. Generally, for a somewhat stable pool, the amounts will be greater than those provided initially (while adding) because of the swap fees.

One might wonder when to use ```#[payment_*]``` macros and when not to use them. In this particular case, the only reason is that when using macros, the endpoint requires that only one payment is provided, and will not accept multiple payments by design, so no additional checks have to be done in the contract. This is the implemented logic throughout all the endpoints of this contract.

### swapTokensFixedInput

```rust
    #[payable("*")]
    #[endpoint(swapTokensFixedInput)]
    fn swap_tokens_fixed_input(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] amount_in: BigUint,
        token_out: TokenIdentifier,
        amount_out_min: BigUint,
    );
```

This smart contract acts as an AMM based on the constant product formula ```x * y = k```.
This means that swapping, when ignoring fees, would happen based on the following logic:

Let:

- __rI__ be the reserve of the input token (the one that the user paid)
- __rO__ be the reserve of the output token (the one that the user desires in exchange of the paid one)
- __aI__ be the amount of the input token
- __aO__ be the amount of the output token

```math
rI * rO = k
(rI + aI) * (rO - aO) = k
```

From the two equations, we can safely state that

```math
rI * rO = (rI + aI) * (rO - aO)
```

Where __aI__ would be known, and __aO__ would need to be calculated.

Considering __f__ being the percent of total fee, the formula including fees is the following:

```math
rI * rO = (rI + (1 - f) * aI) * (rO - aO)
```

Let's presume that __aO__ will be calculated only by taking into account ```(1 - f) * aI```. In Maiar Exchange's current configuration, that would be 0.97% of the input.

The remaining fee, which is ```f * aI``` would be split afterwards into regular fee - reinvested in the pool and special fee - used for buyback and burn mex. For more in depth dive into how the special fee is handled, see ```send_fee``` private function.

### swapTokensFixedOutput

```rust
    #[payable("*")]
    #[endpoint(swapTokensFixedOutput)]
    fn swap_tokens_fixed_output(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] amount_in_max: BigUint,
        token_out: TokenIdentifier,
        amount_out: BigUint
    );
```

Happens exactly the same as SwapFixedInput function with the only difference that __aO__ is fixed and __aI__ is calculated using the same formulas.

One other difference is that the contract actually returns the desired tokens to the users, and also the __leftover__, in case there is any.

The __leftover__ in this case is the difference between the __amount_in_max__ and the actual amount that was used to swap in order to get to the desired __amount_out__.

### swapNoFeeAndForward

```rust
    #[payable("*")]
    #[endpoint(swapNoFeeAndForward)]
    fn swap_no_fee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] amount_in: BigUint,
        token_out: TokenIdentifier,
        destination_address: ManagedAddress,
    );
```

This endpoint performs a swap of tokens with no fee. It is a public endpoint but it requires whitelisting. This endpoint is meant to be used by other pair contracts that need to Swap tokens to MEX so that they can Burn it everytime a swap has happened.

### removeLiquidityAndBuyBackAndBurnToken

```rust
    #[payable("*")]
    #[endpoint(removeLiquidityAndBuyBackAndBurnToken)]
    fn remove_liquidity_and_burn_token(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_nonce] nonce: u64,
        #[payment_amount] amount_in: BigUint,
        token_to_buyback_and_burn: TokenIdentifier,
    );
```

This endpoint is used to convert LP tokens into MEX and then burn it. The way it works is: it performs a remove liquidity action, then swaps (if needed) each of the two tokens into mex (swapping is done also at zero fee). This endpoint is meant to be used by the farm contracts for burning penalties. When penalties need to be applied, the farm doesn't just burn the LP tokens, instead it uses this endpoint to buyback and burn mex, thus helping the product and the ecosystem.

## Testing

There are four test suites around this contract:

- __mandos__ tests are located in the _dex/mandos_ directory. The tests can be ran using __mandos-test__
- __rust__ tests are written using __rust_testing_framework__. The tests can be ran as any other rust tests using __cargo-test__. This test suite is to be preferred and will be extended and maintained over the mandos tests because the testing framework offers programmatic testing.
- __fuzzing__ tests are located in the _dex/fuzz_ directory. The tests can also be ran using __cargo-test__
- __legolas__ tests are python scripts that use actual live transactions on testing setups

## Interaction

The interaction scripts are located in the _dex/interaction_ directory. The scripts are written in python and erdpy is required in order to be used. Interaction scripts are scripts that ease the interaction with the deployed contract by wrapping erdpy sdk functionality in bash scripts. Make sure to update the PEM path and the PROXY and CHAINID values in order to correctly use the scripts.

## Deployment

The deployment of this contract is done by the Router smart contract but it can be done also in a standalone manner using the erdpy interaction scripts for deploy/upgrade.
