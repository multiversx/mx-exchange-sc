# Pair Smart Contract

This document presents how one can deploy and configure a Pair contract.
The bigger picture about what a Pair contract can do can be found in the Repository's Root Readme.

## Deployment

There are two ways of deploying this contract:

- Through Router contract (The way used in Maiar Exchange)

- Deployed directly using a tool like `erdpy`

The init parameters are:

- first_token_id. First token of the Pair Tokens

- second_token_id. Second token of the Pair Tokens

- Router Address. In case no router is used, can be replaced with the owner address

- Router Owner Address. Same as for Router Address

- Total Fee Percent. Must be a number between 0 (0%) and 99_999(99.999%). This is the total fee applied to each swap

- Special Fee percent. Must be a number between 0 (0%) and Total Fee Percent. It's the fee that will not remain in the pool (ie. will can be burned or send somewhere else).

## Interraction

The general DEX erdpy snippet file covers most of the endpoints and views an admin or a user might be interested in calling.

## Special Fee Handling

The fee that will not remain in the contract can be configured in multiple ways. The fee.rs module contains a `fee_destination` which is a map of Address and TokenId. The contract will try to split one transaction fee to all the addresses in the fee_destination equaly. If the token type requested by an address in the fee_destination does not match either of the tokens locally, the contract will try to resolve this by doing an external swap. An external swap is when a Pair needs TokenC (because it was requested by AddressA within fee_destination) and it only has TokenA and TokenB available in the pool. The contract will try to do at most one external transfer TokenA to TokenC or TokenB to TokenC in order to be able to send the fee as configured. Within the fee.rs module, there's a storage named `trusted_swap_pair` that will contain the addresses where it's safe to ask for swaps. These external swaps will happen with 0 fees.

Configuring a Pair contract to send fee tokens to `Address::zero()` will result in burning of the tokens.

A pair only allows certain addresses to use the external swap with no fees, otherwise, all users might have called the same endpoint in order to avoid the fees. A pair knows what addresses can call the endpoint by storing them in `whitelist` storage.

## Roles

The Pair should have at least LocalMint and LocalBurn roles for the LP Token. Those roles should be set by either Router SC or by the user manually. In addition, if fee is disired to be burned, the LocalBurn role should be granted for the specific token type.
