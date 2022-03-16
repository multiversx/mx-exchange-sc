# Price Discovery

## Introduction

The Price Discovery smart contract used to determine the price of a certain token at its launch. Since it’s a new token, the demand for it is not yet known, so we let the community decide.

## Basic concepts

We define two tokens, one called the _launched_ token, which is the new token, and an already existing/established token, called the _accepted_ token. The price discovery SC will accept deposits/withdrawals of both tokens for a certain period of time, in which the price of the tokens will fluctuate.

Once that period has ended, it will deposit those tokens inside a Liquidity Pool, and receive LP tokens (Liquidity Pool tokens), which can then be used to withdraw tokens from the pool. Users that deposited tokens in the previous phase will be elligible to receive a fraction of the LP tokens, based on their contribution to the pool's initial liquidity.  

## Phases

Over the start-end period, we define multiple phases, in which interactions with the Price Discovery SC will impose some restrictions:
1) No restrictions. Anyone can deposit/withdraw any amount
2) Deposits are unrestricted, withdrawals come with a linear increasing penalty
3) Deposits are not allowed, withdrawals come with a fixed penalty
4) Neither deposits nor withdrawals are allowed.

During phase 4, the _accepted_ and _launched_ tokens are deposited into the liquidity pool. After a certain amount of epochs (also known as unbond period), users will be able to withdraw their fair share of the LP tokens, based on their contribution to the initial liquidity.

Accumulated penalties during phase 2 and 3 will be part of the initial liquidity added to the LP on phase 4. So when users will come and will get into redeem period, they will get more LP tokens. As the current quantity of metaESDT represents a percentage of the total liquidity. The users who withdrawn with penalty - they already got their token A or token B, with penalty applied - so they got less than deposited. This is part of the basic process of LP computation.

## Minimum price

The minimum price for the launched tokens can be set by the owner and can be set in the init period of the contract, before entering phase 1.The first deposit has to be with the _launched_ token, otherwise minPrice invariant is not sustained.

The minPrice check is done at the END of every deposit and withdraw function independent on the phase (1, 2, 3). This has a set of implications: if the price gets too low, users will not be able to withdraw their accepted token (token B - wEGLD). The same goes into deposit as well - users will not be able to deposit launched tokens if there is not enough liquidity of accepted tokens (which means price is too low).

## Extra rewards

The owner of the price discovery contract can set a token ID which is accepted as reward in the contract. After that any address can deposit rewards to the contract on phase 1,2,3,4. At phase 5, when the redeem endpoint is open, the users will get a share of the deposited rewards. These rewards are an incentive to the initial liquidity providers who blocked their fungible assets for the given period.
