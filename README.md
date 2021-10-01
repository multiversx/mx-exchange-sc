# Maiar Exchange Smart Contracts

This repository contains the principal Smart Contract components of Maiar Exchange.

This document will contain an overview of the Smart Contracts. It covers the basic workflows that a user may do in order to successfully interact with each contract. For those interested in more in-depth technical details, each contract will have its separate README in its own root directory.

- [Maiar Exchange Smart Contracts](#maiar-exchange-smart-contracts)
  - [DEX Contracts](#dex-contracts)
    - [Pair Contract](#pair-contract)
      - [Adding liquidity](#adding-liquidity)
      - [Removing liquidity](#removing-liquidity)
      - [Swapping](#swapping)
    - [Router Contract](#router-contract)
    - [Farm Contract](#farm-contract)
      - [Entering Farm](#entering-farm)
      - [Exiting Farm](#exiting-farm)
      - [Claiming rewards](#claiming-rewards)
  - [MEX Distribution Contracts](#mex-distribution-contracts)
    - [Distribution Contract](#distribution-contract)
    - [DEX Proxy Contract](#dex-proxy-contract)
      - [Proxy Pair Module](#proxy-pair-module)
      - [Proxy Farm Module](#proxy-farm-module)
  - [Locked MEX Factory Contract](#locked-mex-factory-contract)

Other Smart Contracts that are part of Maiar exchange, but are not part of this repository, are:

- [Egld wrapping](https://github.com/ElrondNetwork/sc-bridge-elrond/tree/main/egld-esdt-swap) used for swapping EGLD to an ESDT and reversed with an exchange rate of 1:1.
- [Multisig](https://github.com/ElrondNetwork/sc-bridge-elrond/tree/main/multisig)
used for deploying and setting up all the Smart Contracts.

## DEX Contracts

These are the core Smart Contracts and the foundation of Maiar Exchange. They consist of three contracts, all of which will be explained below.

Some important general properties:

- Every DEX contract is marked as `non-payable` in order to minimize the risk of a user sending his tokens to the contract by accident and hence locking them indefinitely.

- Every DEX contract is `stateless`, meaning that no contract will keep track of any user data.
  
- Every interaction with the contracts can be done using `erdpy`. A set of snippets is provided as a starting point for anyone interested.

### Pair Contract

The Pair Contract acts as an AMM for trading two different tokens. The AMM is based on liquidity pools and on the already popular `x*y=k` approach. The total fee percent for each swap can be configured at the init phrase (at deploy time), although the default value (and the value that will be used) will be `0.3%`, from which `0.25%` will stay in the liquidity pool, `0.05%` will be used to buyback and burn MEX tokens.

One technical subtlety of these contracts is that it only functions with `Fungible Tokens`. It handles neither EGLD nor Semi-Fungible tokens (SFTs) nor Non-Fungible Tokens (NFTs).

#### Adding liquidity

A user can become a Liquidity Provider by adding liquidity to the pool. For doing that, a user has to transfer tokens for both types to the contract. By doing this, the user will receive an `LP token`, which will represent his position in the liquidity pool. From now on, the user will gain a part of the fees accumulated by swap transactions, until of course, he decides to exit the liquidity pool.

A user can sell his position or part of his position in the pool at his own willing. This can be done by simply transferring LP tokens to another account. The LP token is a fungible token and like any other ESDT token, it can be transferred directly anywhere without the need to interact with any contract.

#### Removing liquidity

A Liquidity Provider may decide to exit a liquidity pool. In order to do this, the user has to give back his LP tokens and in exchange, he will get his tokens + his rewards (share of swap fees).

The amount of each type of token that will receive depends on the current state of the liquidity pool, and specifically on the swap ratio. Hence, if the ratio in the pool changes by the time a user added liquidity when deciding to remove liquidity, a user will receive tokens only based on the current ratio of the pool and not based on the ratio from when he added liquidity.

#### Swapping

The primary functionality that the Pair Contracts offers is allowing users to swap tokens. The user can configure a minimum amount of tokens that he wants to get back in exchange for his fixed amount of input tokens, and also he can configure the maximum amount of tokens that he wants to spend in order to get a fixed amount in exchange. This is particularly useful because a FrontEnd may use these parameters to expose a slippage percent to the user, much like we already saw in a lot of exchanges.

A fee of 0.3% will be deducted from each swap. Part of each swap (0.25%) fee will go to the Liquidity Providers and the other part (0.05%) will be used to buyback and burn MEX tokens.

### Router Contract

The Router Contract is a manager for the pair contracts. All the Pair Contracts in this DEX will be deployed through the router.

It was the permission to configure pair contracts and to set their state. It also contains the Table where a user can query the contract address for a specific swap pair.

It's also meant to be used by any user to create and deploy a Pair Contract, although there is a setting to turn this option On and Off.

### Farm Contract

The Farm Contract is a contract where a user can block his tokens in order to receive rewards. It is similar to the staking mechanism, where you lock your EGLD to receive more EGLD, but still, there are a lot of differences between the two and should not be confused which each other.

The differences include:

- The token that you use in order to be able to enter a Farm is not necessary the same as the reward token. Hence, in the contract we named the first one `Farming token` and the second one `Reward token`.

- The farm position is always tokenized. This allows the farm contract to be stateless in the sense that no user information will be held in the contract. When entering a farm, a `Farm Token` will be granted to the user, and it will represent his position in the Farm. This farm token can be used to claim rewards and to get back the Farming Tokens. The number of farming tokens that he will get back are the same as the amount that he entered.

- There are two possible sources of rewards. First is from the swap fees. The pair will automatically send the fees in the requested token type (MEX tokens) to the farm and Farm's responsibility is to distribute them fairly between the farmers. The other source of rewards is by minting. The contract can also mint per block rewards, this value being configurable.

- The Farm position, represented by the Farm Token, will be a Semi-Fungible Token. The reasoning behind this is that in order to correctly calculate the reward each farmer should get, some metadata needs to be stored in the SFT's attributes.

- The Farm position, similar to the liquidity pool position, can be sold by just transferring the SFTs to another account without any need to interact with any contract.

#### Entering Farm

Entering a Farm is done by a user transferring his Farming tokens to the farm. By doing this, the user will receive Farm Tokens, which he can use to both claim rewards and get back his tokens, by exiting farm.

One thing to mention here is that a user can opt for receiving his MEX Reward as Locked MEX instead of regular MEX. By doing so, a user will benefit from `Double APR` when it comes to rewards.

#### Exiting Farm

Existing a Farm is done by a user transferring his Farm Tokens to the farm. By doing this, a user will automatically get his reward and his farming tokens back.

On constraint here is that:

- A user cannot exit farm earlier than `3 epochs` after he entered if the user went for the Double APR option. This is because these rewards are designed to be long-term rewards hence entering and exiting with this option too frequently should be discouraged.

- A user that did not go for the long-term reward can exit farm at any time, but exiting before 3 epochs were reached after entering will be penalised with 10% of both rewards and farming tokens.

Entering a farm should be a proof that a user wants to get more tokens by locking his own for at least a minimum amount of epochs.

#### Claiming rewards

Claiming rewards will be done by a user transferring his Farm Tokens to the farm. By doing this, a user will get his Reward Tokens for the period starting with either EnterFarm or the last ClaimRewards operation, plus another Farm Token. The reason behind sending a Farm Token and receiving another one is in order to place a new reward counter in the newly created SFT and burn the old SFT, for which the rewards have been claimed.

## MEX Distribution Contracts

### Distribution Contract

The Distribution Contract will be used in order to distribute the first `MEX tokens` to the community. The particular thing about this is that the first tokens will be in the form of `LOCKED MEX`.

This contract can receive information from the snapshots of eGLD holder and will keep track of how many Locked MEX should each user get. The only functionality of this contract is allowing the user to come and claim his distribution share of Locked Mex through the Locked MEX Factory Contract.

### DEX Proxy Contract

The DEX Proxy Contract can be used by the user to interact with the DEX contracts with Locked MEX. The principal idea is that a user can use his Locked MEX as if it was MEX for adding liquidity.

There are two major components in this contract and we'll explain them separately.

#### Proxy Pair Module

Proxy Pair Module allows a user to add and remove liquidity using Locked MEX in the pair contracts that have MEX as one of the pair tokens.

This contract is also stateless, will not keep track of any -per-user-data- and the position in the pools will be also tokenized. By entering a liquidity pool with a regular token and a Locked MEX, a user will receive `Wrapped LP Tokens`. The Wrapped LP Token, as the name suggests, is just a Wrapper over the regular LP Token, and it exists because a user cannot receive the LP Tokens directly, otherwise, he could remove liquidity and get back MEX tokens, instead of his Locked MEX provided.

Trading Wrapped LP Tokens can be done by transferring directly. The Wrapped LP Token can be used for either removing liquidity or for entering a Farm.

#### Proxy Farm Module

Proxy Farm Module allows a user to enter a Farm with Wrapped LP Tokens. A user can also enter a Farm that accepts MEX as Farming token using Locked MEX. By entering a Farm with either Wrapped LP Token or Locked Mex, a user will receive `Wrapped Farm Tokens`. The user can use those tokens to claim rewards or to exit the Farm, which will result in him getting back the tokens used to enter farm via the proxy (so either Wrapped LP Token or Locked MEX) and his rewards (either MEX or Locked MEX).

## Locked MEX Factory Contract

This contract is the only contract that has the ability to create `LOCKED MEX` tokens. Other contracts (that are whitelisted) can request creating and forwarding of these tokens.

Locked MEX is an SFT Token. The reasoning behind this is because each Locked Mex, depending on the creating parameters, can have a different `Unlock Schedule`.

Each `Locked MEX` has an unlock schedule because its goal is to represent a MEX that can only be unlocked in the future. This unlock will not happen once, it will be in different phases, configurable. For example, it can be configured to be unlocked with for example 10% every month.
