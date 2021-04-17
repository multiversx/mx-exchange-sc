# Dex overview

This document will describe how the Dex is going to work and what role
each contract within it has. Let's say we have the next set of contracts
up and running:

```
EGLD/MEX pair contract for swap
EGLD/BUSD pair contract for swap
MEX/BUSD pair contract for swap
Staking pool for EGLD LP providers
Staking pool for MEX LP providers
```

## Most common scenarios

### Adding Liquidity

Let's say Alice wants to add liquidity to EGLD/MEX pool. She will need
to provide EGLD and MEX to the EGLD/MEX pair contract. In return, she
will get LP tokens (tokens that will represent her position in the
liquidity pool), let's call them LP-EGLD-MEX tokens. These tokens are
normal ESDT tokens, so called fungible tokens.

One important thing to mention here is that the position in a liquidity
pool can be traded also. So Alice can decide to sell her LP-EGLD-MEX
tokens to Bob and then Bob will be the owner of the position in the
liquidity pool (meaning that as long as he's got those LP-EGLD-MEX tokens,
he can decide to get out of the pool at any time).

### Swapping with fixed input

Let's say Bob wants to swap 1000 EGLD for MEX. He will give 1000 EGLD to
EGLD/MEX pair contract and also specify the minimul amount of MEX that
he's expectig. On success, he will receive an amount of MEX, greater or
equal with the minimul amount of MEX that he requested. On failure, he
will get back his 1000 EGLD. About the fees now, 0.3% is the total fees
of the swap, so a total of 3 EGLD. From that, 0.2% is going to stay in the
liquidity pool, so 2 EGLD goes back, and will be claimed by the liquidity
providers. The rest of 0.1%, so 1 EGLD, will go to the staking contracts.
Since there are 2 staking contracts, 0.5 EGLD will go to EGLD Staking
contract and 0.5 EGLD will go to the MEX Staking contract.

One important thing to mention is that the MEX Staking contract works
with MEX as rewards, so the 0.5 EGLD will actually have to be swapped to
MEX in order to be sent to the MEX Staking contract. In case of EGLD/MEX
pair contract, the swap can be done locally, within the same pair contract.
In case of other contracts, like EGLD/BUSD contract, the BUSD fees from
swaps will have to be converted externally, so the EGLD/BUSD makes a swap
request to BUSD/MEX pool. This process is automated and invisible to the
user. The swap of fees, either locally or externally will happen with 0 fees.

### Swapping with fixed output

Swapping with fixed output works exactly the same as Swapping with fixed
input, except that you specify the precise amount of tokens you want to get,
and you pay the maximum amount that you are willing to pay for the swap. In 
case you pay 10 EGLD and you want 20 MEX, but the price of these 20 MEX swap
was actually 8 EGLD, you will get back the 2 extra EGLD that you provided + 
the 20 MEX tokens. About the fees, they work in a similar manner as explained
above.

### Removing liquidity

Let's say Alice wants to get out of EGLD/MEX liquidity pool. She will have to
give some of her LP-EGLD-MEX tokens and she will receive EGLD and MEX amounts
calculated at the current time. She will benefit of 0.2% fee from all swaps 
within the period she provided for the pool (calculated for her position in the
pool, of course).

### Staking 

Let's say Alice added some liquidity to EGLD/MEX liquidity pool and she
wants to stake her LP-EGLD-MEX tokens. She will have to go to EGLD Staking
contract (or MEX Staking contract, or both in case of LP-EGLD-MEX) and lock
her LP-EGLD-MEX tokens. In return, she will get EGLD-STAKE tokens. These
tokens are Semi-Fungible tokens and they contain information needed when
unstaking (and calculating rewards).

One important thing to mention is that Alice can sell her position in the
Staking pool. The meaning and reasonings are explained in the "Add Liquidity"
section.

### Unstaking

Let's say Alice wants some of her LP tokens back, she will have to provide
some EGLD-STAKE tokens to the EGLD staking contract. In return, she will
get some EGLD rewards for the time spent as an EGLD staker, and some Unstake
tokens, let's call the EGLD-UNSTAKE tokens. These tokens are also Semi-Fungible
tokens and they contain the information needed for unbonding (most importantly
here is the unbond period, which is 10 days).

One important thing to mention here is that we introduced this EGLD-UNSTAKE
token so the EGLD staking contract will not have to keep track of any
addresses. Pancake Swap keeps track of Users that Unstaked, meaning that they
have a per user address mapping that keeps information like the unbonding
epoch. In order to make the staking contract fully decentralized and stateless,
we introduced this unstake token that has that kind of information. The staking
contracts will have no "per user mapping". :) 

### Unbonding

Let's say Alice unstaked some of her EGLD-STAKE tokens and she got some
EGLD-UNSTAKE tokens and 10 days passed since then. Alice can give her 
EGLD-UNSTAKE and she will receive back her LP-EGLD-MEX tokens.


