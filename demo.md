## Demo

For the purpose of demonstrating how everything works, we made the setup
described in the README.md (3 liquidity pool pair contracts and 2 staking pool
contracts) and we provided some snippets to help interracting with them.
Everything is deployed on public testnet (Staking contracts are deployed
with a 0 unbonding period).

```
EGLD/MEX pair contract for swap address: erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4
EGLD/BUSD pair contract for swap address: erd1qqqqqqqqqqqqqpgqythkz66phnu9cx6dvhj7f6x7vyqklkrmephq5kx34d
MEX/BUSD pair contract for swap address: erd1qqqqqqqqqqqqqpgqzvnj3wkhm2x7hn7zvn3rsflhdyjw5j5mephq96pam9
Staking pool for EGLD LP providers address: erd1qqqqqqqqqqqqqpgqyfg8hntu4c3hunh0p6zz8xt7w42aqs79ephqgzwn2g
Staking pool for MEX LP providers address: erd1qqqqqqqqqqqqqpgq35yj7rawq96fqmyj3ravzt6jl4z8579kephqjh3p0q
```

```
EGLD token id: EGLD-bf16ca
MEX token id: MEX-0a3d3a
BUSD token id: BUSD-56c018
LP-EGLD-MEX token id: LPT-8ad0eb
LP-EGLD-BUSD token id: LPT-3a0e12
LP-MEX-BUSD token id: LPT-714aeb
EGLD-STAKE token id: STEGLD-2e6833
EGLD-UNSTAKE token id: UNSTEGLD-9b0ff6
MEX-STAKE token id: STMEX-4cca05
MEX-UNSTAKE token id: UNSTMEX-044c48
```

We can provide some EGLD, MEX, BUSD (please remember this is on testnet) so you can add/swap/remove, stake/unstake/unbond.

## Snippets usage examples:

### Adding liquidity
In order to add liquidity to EGLD/MEX (say 10000EGLD and 10000MEX, with minimum
accepted 9990EGLD 9900MEX),
one might do this steps:
```
transferTokens erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4 EGLD-bf16ca 0x2710
transferTokens erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4 MEX-0a3d3a 0x2710
addLiquidity erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4 0x2710 0x2710 0x2706 0x26AC
reclaimTemporaryFunds erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4
```

### Swapping with fixed input
In order to swap EGLD for MEX (say 100EGLD for minimum 10MEX), one might use this:
```
swapFixedInput erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4 EGLD-bf16ca 0x64 MEX-0a3d3a 0x0a
```

### Swapping with fixed output
In order to swap EGLD for MEX (say maximum 100EGLD for 10MEX), one might use this:
```
swapFixedOutput erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4 EGLD-bf16ca 0x64 MEX-0a3d3a 0x0a
```

### Removing liquidity
In order to remove liquidity (say he want to return 255 LP-EGLD-MEX tokens, and wants
minimul 10EGLD and 100MEX), one might to this step:
```
removeLiquidity erd1qqqqqqqqqqqqqpgqqwvkms3v7gjuapklycf56cmnkw2g4nt4ephq5uccm4 LPT-8ad0eb 0xff 0x0a 0x64
```

### Staking
In order to stake (say we have 2000 LP-EGLD-MEX tokens), one might do this step:
```
stake erd1qqqqqqqqqqqqqpgqyfg8hntu4c3hunh0p6zz8xt7w42aqs79ephqgzwn2g LPT-8ad0eb 0x07d0
```

### Unstaking
In order to unstake (say we have 100 EGLD-STAKE tokens with nonce = 1), one might do this step:
```
unstake STEGLD-2e6833 0x01 0x64 erd1qqqqqqqqqqqqqpgqyfg8hntu4c3hunh0p6zz8xt7w42aqs79ephqgzwn2g
```

### Unbonding
In order to unstake (say we have 100 EGLD-STAKE tokens with nonce = 1), one might do this step:
```
unbond UNSTEGLD-9b0ff6 0x01 0x64 erd1qqqqqqqqqqqqqpgqyfg8hntu4c3hunh0p6zz8xt7w42aqs79ephqgzwn2g
```

In addition to this examples, there are more functions that are exposed in the
snippets file. Part of them were used for the setup but others can be used at
anythime, like for example the contract querries. You can also deploy your own
contracts and setup your own dex in any manner you want. Setting up a new dex
might involve a lot of steps that were not described in this document for the
sake of keeping everything as simple as possible but we can offer support in
case there are questions or setup problems.

