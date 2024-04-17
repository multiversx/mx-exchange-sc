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
- __special_fee_percent__ - The total fee is handled in two different manners. By default, the fee is reinvested into the pool, thus increasing the amount of tokens of liquidity providers. Part of the total fee is handled differently. More specific, a value of 50, representing a percentage of 0.05 of each swap is used to buyback and burn MEX tokens. The special fee can also be split in many equal parts and can be configured to be sent to other addresses also. The convention is that, if a configured special fee destination address is zero, then the tokens should be burned (the way it is configured in xExchange as well).
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

Let's presume that __aO__ will be calculated only by taking into account ```(1 - f) * aI```. In xExchange's current configuration, that would be 0.97% of the input.

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

- __scenario__ tests are located in the _dex/scenarios_ directory. The tests can be ran using __mandos-test__
- __rust__ tests are written using __rust_testing_framework__. The tests can be ran as any other rust tests using __cargo-test__. This test suite is to be preferred and will be extended and maintained over the scenario tests because the testing framework offers programmatic testing.
- __fuzzing__ tests are located in the _dex/fuzz_ directory. The tests can also be ran using __cargo-test__
- __legolas__ tests are python scripts that use actual live transactions on testing setups

## Interaction

The interaction scripts are located in the _dex/interaction_ directory. The scripts are written in python and mxpy is required in order to be used. Interaction scripts are scripts that ease the interaction with the deployed contract by wrapping mxpy sdk functionality in bash scripts. Make sure to update the PEM path and the PROXY and CHAINID values in order to correctly use the scripts.

## Deployment

The deployment of this contract is done by the Router smart contract but it can be done also in a standalone manner using the mxpy interaction scripts for deploy/upgrade.


# Safe Price V2

## General overview

The new safe price mechanism is an updated module of the __Pair SC__, designed to stabilize prices over a number of rounds, creating a smoother, more predictable pricing pattern in the __xExchange__ liquidity pools. The mechanism achieves this by storing accumulated reserves over time, with each recorded round representing a snapshot of token reserves at that point in time. This collection of reserve data is then processed through a variety of algorithms to compute and retrieve the safe price. Notably, the mechanism operates through a central view factory contract, which manages requests for all active smart contract pairs. The view factory contract accepts the address of the desired pair as a parameter in its view functions, simplifying the process of querying for data.

__Important. The Safe Price module retrieves data independently of the liquidity pool's state. As such, even if a pair smart contract is paused, the safe price module will continue to return data. If an external contract requires more control or context concerning the data, it needs to first check the liquidity pool's status manually and subsequently utilize the safe price data according to its needs.__

## Features

- __Accumulated Reserves:__ The mechanism consistently stores the accumulated reserves over time. Each round in which the liquidity pool contract is used creates a snapshot of the token reserves for that time, with the weight of a __PriceObservation__ for a round being the difference between the last saved round and the current one.
- __Price Observations:__ Each round's reserves, once calculated and stored, are then saved in a __PriceObservation__ struct for the subsequent round (n+1). This allows for a clear record of price changes and liquidity over time.
- __Circular List Storage and Binary Search:__ Price observations are stored in a circular list, which is an efficient data structure for storing the rolling price data. A binary search algorithm is used to find specific __PriceObservations__ in this list.
- __Linear Interpolation:__ If a price observation is not available for a queried round, the algorithm will perform a linear interpolation between the nearest price observations to estimate the price for that round.
- __Error Handling:__ To maintain data integrity, a query for a price observation older than the oldest stored observation will result in a SC error. This mechanism helps to prevent the use of outdated or non-existent data.
- __Versatile Safe Price Request Inputs:__ The mechanism offers several view functions, each providing a different way to query the safe price. These views give users flexibility in querying the safe price by either providing all necessary parameters or using default ones. 

## Endpoints available on the View factory contract

### getSafePrice

```rust
    #[view(getSafePrice)]
    fn get_safe_price(
        &self,
        pair_address: ManagedAddress,
        start_round: Round,
        end_round: Round,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment
```

This function allows you to retrieve the safe price by specifying the pair address, start round, end round, and input payment. 
It returns the corresponding output payment computed at the safe price.

### getSafePriceByRoundOffset

```rust
    #[view(getSafePriceByRoundOffset)]
    fn get_safe_price_by_round_offset(
        &self,
        pair_address: ManagedAddress,
        round_offset: u64,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment
```

This function allows you to retrieve the safe price by specifying the pair address, round offset, and input payment. It calls the generic __getSafePrice__ endpoint, by automatically computing the end_round parameter as the current block round, and the start_round as the difference between the current_round and the provided round_offset.
It returns the corresponding output payment computed at the safe price.

### getSafePriceByDefaultOffset

```rust
    #[view(getSafePriceByDefaultOffset)]
    fn get_safe_price_by_default_offset(
        &self,
        pair_address: ManagedAddress,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment
```

This function allows you to retrieve the safe price by specifying the pair address and input payment. It works in the same way as __getSafePriceByRoundOffset__ endpoint, but instead of using a provided round_offset parameter, it uses a default one, which at this moment is set to 600 rounds (which translates in an one hour window).
It returns the corresponding output payment computed at the safe price.

### getSafePriceByTimestampOffset

```rust
    #[view(getSafePriceByTimestampOffset)]
    fn get_safe_price_by_timestamp_offset(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset: u64,
        input_payment: EsdtTokenPayment,
    ) -> EsdtTokenPayment
```

A more specific view function, it allows you to retrieve the safe price by specifying the pair address, a timestamp offset, and input payment. It basically converts the timestamp_offset value to a generic round_offset, by dividing it with the constant value __SECONDS_PER_ROUND__.
It returns the corresponding output payment computed at the safe price.
__Important. The output of this endpoint (and any timestamp related endpoint) will return reliable data as long as the timestamp constant will remain unchanged at the protocol level.__


### getLpTokensSafePrice

```rust
    #[view(getLpTokensSafePrice)]
    fn get_lp_tokens_safe_price(
        &self,
        pair_address: ManagedAddress,
        start_round: Round,
        end_round: Round,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>
```

This function allows you to simulate the value of both tokens within a liquidity pool based on a provided LP token amount. It receives the pair address, the start and end rounds, and the amount of LP tokens as parameters.
The function returns two output payments, one for each token in the pair, with their values computed at the safe price.

### getLpTokensSafePriceByRoundOffset

```rust
    #[view(getLpTokensSafePriceByRoundOffset)]
    fn get_lp_tokens_safe_price_by_round_offset(
        &self,
        pair_address: ManagedAddress,
        round_offset: Round,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>
```

This function allows you to simulate the value of both tokens within a liquidity pool based on a provided LP token amount. It receives the pair address, a round_offset, and the amount of LP tokens as parameters. It works in the same way as the __getSafePriceByRoundOffset__ function for the round_offset, by computing the end round and the start round automatically, using the current block round and the round offset variable.
The function returns two output payments, one for each token in the pair, with their values computed at the safe price.

### getLpTokensSafePriceByDefaultOffset

```rust
    #[view(getLpTokensSafePriceByDefaultOffset)]
    fn get_lp_tokens_safe_price_by_default_offset(
        &self,
        pair_address: ManagedAddress,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>
```

This function allows you to simulate the value of both tokens within a liquidity pool based on a provided LP token amount. It receives the pair address and the amount of LP tokens as parameters and works similarly to __getLpTokensSafePriceByRoundOffset__ endpoint, but instead of using a provided round_offset variable, it uses a default one which at this moment is set to 600 rounds (which translates in an one hour window).
The function returns two output payments, one for each token in the pair, with their values computed at the safe price.

### getLpTokensSafePriceByTimestampOffset

```rust
    #[view(getLpTokensSafePriceByTimestampOffset)]
    fn get_lp_tokens_safe_price_by_timestamp_offset(
        &self,
        pair_address: ManagedAddress,
        timestamp_offset: u64,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment, EsdtTokenPayment>
```

This function allows you to simulate the value of both tokens within a liquidity pool based on a provided LP token amount. It receives the pair address, a timestamp_offset and the amount of LP tokens as parameters. Again, like the __getLpTokensSafePriceByRoundOffset__ endpoint, which automatically computes the start and end rounds of the query, this function calculates the round offset by dividing the timestamp_offset to a generic __SECONDS_PER_ROUND__ constant value.
The function returns two output payments, one for each token in the pair, with their values computed at the safe price.

## Legacy endpoints

In order to avoid backwards compatibility issues, the two legacy endpoints from Safe Price V1 were kept, but they now use the new Safe Price V2 logic. One important aspect here is that they are not part of the Safe Price V2 view factory contract, but instead they are actual endpoints in the __Pair SC__.
__Important. These endpoints are planned to be phased out at any point in future, thus we consider them as deprecated and we recommend the usage of the ones available in the View factory contract.__

### updateAndGetTokensForGivenPositionWithSafePrice

```rust
    #[endpoint(updateAndGetTokensForGivenPositionWithSafePrice)]
    fn update_and_get_tokens_for_given_position_with_safe_price(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>
```

This legacy endpoint is kept for backwards compatibility purposes, but it now works the same as the __getLpTokensSafePriceByDefaultOffset__ view function, by using the contract address as the pair_address. It receives only one parameter, the amount of LP tokens.
The function returns two output payments, one for each token in the pair, with their values computed at the safe price.

### updateAndGetSafePrice

```rust
    #[endpoint(updateAndGetSafePrice)]
    fn update_and_get_safe_price(
        &self,
        input: EsdtTokenPayment<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api>
```

This legacy endpoint is kept for backwards compatibility purposes, but it now works the same as the __getSafePriceByDefaultOffset__ view function, by using the contract address as the pair_address. It receives only one parameter, the input payment.
It returns the corresponding output payment computed at the safe price.
