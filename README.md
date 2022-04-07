# Maiar Exchange Smart Contracts

This repository contains the principal Smart Contract components of Maiar Exchange.

This document is a brief description of the Smart Contracts. It covers the basic workflows that a user may do in order to succesfully interact with each contract. For those interested about more in-depth technical details, each contract will have its separate  README in its own root directory.

- [Maiar Exchange Smart Contracts](#maiar-exchange-smart-contracts)
  - [DEX Contracts](#dex-contracts)
    - [Pair Contract](#pair-contract)
    - [Router Contract](#router-contract)
    - [Farm Contract](#farm-contract)
    - [Farm with Lock Contract](#farm-with-lock-contract)
    - [Price discovery](#price-discovery)
  - [Farm Staking Contracts](#farm-staking-contracts)
    - [Farm Staking](#farm-staking)
    - [Farm Staking Proxy](#farm-staking-proxy)
    - [Metabonding Staking](#metabonding-staking)
  - [Locked Asset Contracts](#locked-asset-contracts)
    - [Distribution Contract](#distribution-contract)
    - [DEX Proxy Contract](#dex-proxy-contract)
    - [Locked MEX Factory Contract](#locked-mex-factory-contract)

Other Smart Contracts that are part of Maiar exchange, but are not part of this repository:

- [Egld wrapping](https://github.com/ElrondNetwork/sc-bridge-elrond/tree/main/egld-esdt-swap) used for swapping EGLD to an ESDT and reversed with an exchange rate of 1:1.

## DEX Contracts

Core SCs of Maiar Exchange. They usually handle few tokens and are used as primitives by the other contract as they are built on top of them.

### Pair Contract

The core of any DEX is the swap feature. This contract acts as a constant product AMM and offers swap functionality aswell as adding & removing liquidity. The fees for swapping are configurable by the Router SC and its owner, although most of the contracts use a total fee percent of 0.3%, from which 0.25% goes to liquidity providers and the rest of 0.05% is used to help the ecosistem buy Buying & Burning MEX tokens.

### Router Contract

The owner of all the Pair SCs is the Router SC. It is used for deploying, upgrading and configuring the Pair SCs. It also holds the mapping between (the pair of the two tokens) and the Contract Address, so when a user wants to swap two assets, he needs to know the Pair SC address for the particular contract. This is the place where this info can be queried.

### Farm Contract

In order to gain users trust, the liquidity inside the DEX must be somewhat stable. To achieve that, Farm contracts come in place to incentivise liquidity providers to lock their LP tokens in exchange for MEX rewards. Rewards are generated per block and their rate is configurable in sync with the MEX tokenomics.

### Farm with Lock Contract

Works the same as the regular Farm with the exception that it does not generate MEX as rewards. Instead, it generates Locked MEX, with the help of the Factory SC. The reason one would go for the locked rewards (LKMEX) instead of the regular rewards (MEX) is that the reward emission rate (reward per block rate) is bigger, meaning the APR is higher.

### Price discovery

In order to improve the expecience of the user, and detrement the use of bots at new launches of new tokens on the DEX, the Price discovery was created. As its name states, this contract aim to find the Market Price of a token even before allowing swaps. This contract is supposed to be used before the creation of a Pair SC that needs this protection. This SC gathers both tokens and add them to a Liquidity Pool as initial liquidity, and the returning LP tokens will be claimable by each user that contributed to the initial step.

## Farm Staking Contracts

TODO

### Farm Staking

TODO

### Farm Staking Proxy

TODO

### Metabonding Staking

TODO

## Locked Asset Contracts

### Distribution Contract

This smart contract is used for ditributing Locked MEX to the community. It receives information from the owner and offers the users posibility to claim the configured amount of tokens. It was used only once and might not be used in the future since we think there are better ways to achieve same functionality and results.

### DEX Proxy Contract

This smart contract allows users to interact with the DEX using Locked MEX as fungible MEX for certain operations, such as adding liquidity and entering Farm contracts.

### Locked MEX Factory Contract

Locked MEX is a Meta ESDT. Since there can be only one address (per shard) that can hold the role of creating a Meta ESDT, this contract was created so the multiple other contracts that need to create Locked MEX can just request them from only one place.
