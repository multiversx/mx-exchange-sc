# Price Discovery

## Introduction

The Price Discovery smart contract is used to determine the price of a certain token at its launch. Since it’s a new token, the demand for it is not yet known, so we let the community decide.

## Basic concepts

We define two tokens, one called the _launched_ token, which is the new token, and an already existing/established token, called the _accepted_ token. The price discovery SC will accept deposits/withdrawals of both tokens for a certain period of time, in which the price of the tokens will fluctuate.

Once that period has ended, users will be able to redeem the opposite token of what they initially deposited (i.e. people that deposited _accepted_ tokens will received _launched_ tokens, and vice-versa).  

## Phases

Over the start-end period, we define multiple phases, in which interactions with the Price Discovery SC will impose some restrictions:

1) No restrictions. Anyone can deposit/withdraw any amount
2) Deposits are unrestricted, withdrawals come with a linear increasing penalty
3) Deposits are not allowed, withdrawals come with a fixed penalty
4) Neither deposits nor withdrawals are allowed.

During phase 4, also known as the _redeem_ phase, the users will be able to redeem tokens based on the current ratio of tokens in the contract. Essentially, users are "buying" the opposite token. Note that during the first few epochs the price discovery ends, the users will receive a locked token instead of the actual token.

Accumulated penalties during phase 2 and 3 will be automatically redeemed by users when claiming. So users will actually buy at a better price than the current ratio if there are a lot of tokens accumulated from penalties.

## Minimum price

The minimum price for the launched tokens can be set by the owner and can be set in the init period of the contract, before entering phase 1. The first deposit has to be with the _launched_ token, otherwise minPrice invariant is not sustained.

The minPrice check is done at the END of every deposit and withdraw function independent on the phase (1, 2, 3). This has a set of implications: if the price gets too low, users will not be able to withdraw their accepted token. The same goes into deposit as well - users will not be able to deposit launched tokens if there is not enough liquidity of accepted tokens (which means price is too low).
