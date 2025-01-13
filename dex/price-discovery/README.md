# Price Discovery

## Introduction

The Price Discovery smart contract is used to determine the price of a certain token at its launch. Since it’s a new token, the demand for it is not yet known, so we let the community decide.

## Basic concepts

We define two tokens, one called the _launched_ token, which is the new token, and an already existing/established token, called the _accepted_ token. The price discovery SC will accept deposits/withdrawals of both tokens for a certain period of time, in which the price of the tokens will fluctuate.

Once that period has ended, users will be able to redeem the opposite token of what they initially deposited (i.e. people that deposited _accepted_ tokens will received _launched_ tokens, and vice-versa).  

## Phases

Over the start-end period, we define multiple phases:

1) Anyone can deposit/withdraw any amount of the accepted token
2) Owner can deposit/withdraw the launched token, but not below _min_launched_tokens_
3) Users can redeem the launched token, while the owner can redeem the accepted token
