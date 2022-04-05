# Pair Smart Contract

## Abstract

The Pair smart contract is used to allow the users to exchange tokens in a decentralized manner.

## Introduction

This contract allows users to provide liquidity and to swap tokens. Users are incentivized to add liquidity by earning fees and by being able to enter farms, thus earning even more rewards. This contract is usually deployed by the router smart contract and it (usually) has no dependency, as it is used as a DeFi primitive.

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
        #[var_args] initial_liquidity_adder: OptionalValue<ManagedAddress>,
    );
```

The init function is called when deploying/upgrading one smart contract. It receives several arguments which the SC cannot function without:

- __first_token_id__ - A Pair smart contract can be for example WEGLD-MEX pair. In this case WEGLD is the first token id, while MEX is the second token id. The order of them matters, and throught the other functions and the code, the clear distinction between them must be made, and they cannot be used interchangably.
- __second_token_id__
- __router_address__ - The address of the router smart contract. Usually it is the case that is the caller address. The router address, together with the router owner address act as managers. They have the rights to upgrade and modify the contract's settings until a multisign dApp implementation will be ready and in place.
- __router_owner_address__
- __total_fee_percent__ - Each time a swap is made, a fee is charged. The total percent is configable by this parameter, but is usually 300, representing a percentage of 0.3, base points being 1_000.
- __special_fee_percent__ - The total fee is handled in two different manners. By default, the fee is reinvested into the pool, thus increasing liquidity providers their amount of tokens. Part of the total fee is handled differently. More specific, a value of 50, representing a percentage of 0.05 of each swap is used to buyback and burn MEX tokens. The special fee can also be split in many equal parts and can be configured to be sent in other addresses also. The convention is that, if a configured special fee destination address is zero, then the tokens should be burned (the way it is configured in Maiar Exchange).
- __initial_liquidity_adder__ - An optional argument that is usually a Price Discovery smart contract. Used to decrease changes and impact of Pump & Dump at new listings.

### addLiquidity

```rust
    #[payable("*")]
    #[endpoint(addLiquidity)]
    fn add_liquidity(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    );
```

Adding liquidity is a one TX process. Initially it was designed to be a 3 TX process composing of: sending the first token, seconding the second token, callin the actual add endpoint. Now that MultiTransfer function is available, the process was simplified by sending both tokens and calling the add liquidity endpoint all at once.

Adding of liquidity never changes the ratio between the two tokens. Considering __rA__ and __rB__ are reserves of the first token and reserves of the second token and __aA__ and __aB__ are the amounts of first token and second token that are desided to be added as liquitity, the following formula has to be true:

```rA / rB = aA / aB``` stating that the ratio of the tokens added to the liquidity pool has to be the same as the tokens that are already in the pool.

Calculating them is easy, since one of the desired values to be added __aA__ and __aB__ can be fixated and the other one comes from the formula above.

First add of liquitity sets the ratio and hence the price for the tokens, because the first add goes though without having to respect any formula, since no tokens are in the pool, so there's no ratio to be taken into account.

As stated above, this function receives two payments, the first token payment and the second token payment. The order matters, so the two payments has to be in the same order as the tokens are in the contract. The arguments of this function are:

- __first_token_amount_min__ - The minimum amounts throught all the endpoints in the contract are used to set the slippage. The way it works is the following: when the formula from above is applied and the resulted __aB__ is bigger than the transferred __aB__, the transferred __aB__ will be fixated and the __aA__ will be calculated with the formula. The resulted __aA__ would have to be between the transferred __aA__ and the __first_token_amount_min__, thus setting the accepted range/slippage.
- __second_token_amount_min__
- __opt_accept_funds_func__ - Throught all the endpoints of the contract, this parameter means the following: if you want to receive any form of payment from this exection via another execution, specify the endpoint name using this parameter. That is the case of non payable contracts that use this smart contract. By being non payable, the Pair smart contract cannot just send the tokens to the caller. Instead, it has to send the tokens by also triggering the execution of some function.

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
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    );
```

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
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    );
```

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
        amount_out: BigUint,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    );
```

## Testing

TODO

## Interraction

TODO

## Deployment

TODO
