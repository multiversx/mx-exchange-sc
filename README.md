# xExchange Smart Contracts

This repository contains the principal Smart Contract components of xExchange.

This document is a brief description of the Smart Contracts. It covers the basic workflows that a user may do in order to succesfully interact with each contract. For those interested about more in-depth technical details, each contract will have its separate README in its own root directory.

- [xExchange Smart Contracts](#xexchange-smart-contracts)
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
  - [Release and reproducible builds](#release-and-reproducible-builds)

Other Smart Contracts that are part of xExchange, but are not part of this repository:

- [Egld wrapping](https://github.com/multiversx/mx-sdk-rs/tree/master/contracts/core/wegld-swap) used for swapping EGLD to an ESDT and reversed with an exchange rate of 1:1.

## DEX Contracts

Core SCs of xExchange. They usually handle a few tokens and are used as primitives by the other contracts as they are built on top of them.

### Pair Contract

The core of any DEX is the swap feature. This contract acts as a constant product AMM and offers swap functionality as well as adding & removing liquidity. The fees for swapping are configurable by the Router SC and its owner, although most of the contracts use a total fee percent of 0.3%, from which 0.25% goes to liquidity providers and the rest of 0.05% is used to help the ecosystem by Buying & Burning MEX tokens.

### Router Contract

The owner of all the Pair SCs is the Router SC. It is used for deploying, upgrading and configuring the Pair SCs. It also holds the mapping between (the pair of the two tokens) and the Contract Address, so when a user wants to swap two assets, he needs to know the Pair SC address for the particular contract. This is the place where this info can be queried.

### Farm Contract

In order to gain users trust, the liquidity inside the DEX must be somewhat stable. To achieve that, Farm contracts come in place to incentivise liquidity providers to lock their LP tokens in exchange for MEX rewards. Rewards are generated based on timestamps and their rate is configurable in sync with the MEX tokenomics.

### Farm with Lock Contract

It works the same as a regular Farm SC, with the exception that it does not generate MEX as rewards. Instead, it generates Locked MEX (XMEX), with the help of the Energy Factory smart contract. Users may choose to receive locked rewards instead of regular MEX because XMEX also provides Energy, which can be used across the entire xExchange protocol.

### Price discovery

In order to improve the experience of the user, and decrease the impact of trade bots at launches of new tokens on the DEX, the Price discovery mechanism was created. As its name states, this contract aims to find the Market Price of a token even before allowing swaps. This contract is supposed to be used before the creation of a Pair SC that needs this protection. The SC gathers both types of tokens and gives each user the corresponding tokens, assets that are locked for a small period of time, in order to further alleviate any unnatural price fluctuations that may appear at launch time.

## Farm Staking Contracts

Staking contracts that are heavily inspired by the farm contracts.

### Farm Staking

Uses the same base implementation and concepts as the Farm contracts, but is designed to work with any fungible tokens instead of very specific ones, like LP tokens. Also, rewards are not minted, but are instead deposited from time to time by the owner.

### Farm Staking Proxy

Used in conjunction with the Farm Staking contract, it lets users stake their LP FARM tokens, i.e. the farm tokens they received when they put their LP tokens in the normal farm. The user then receives a so-called dual yield token, which can be used to claim rewards from both the normal farm and the staking farm.

### Metabonding Staking

A simple staking contract where users lock their tokens in order to receive rewards from other contracts.

## Locked Asset Contracts

### Distribution Contract

This smart contract is used for distributing Locked MEX to the community. It receives information from the owner and offers the users the possibility to claim the configured amount of tokens. It was used only once and might not be used in the future since we think there are better ways to achieve same functionality and results.

### DEX Proxy Contract

This smart contract allows users to interact with the DEX using Locked MEX as fungible MEX for certain operations, such as adding liquidity and entering Farm contracts.

### Locked MEX Factory Contract

Locked MEX is a Meta ESDT. Since there can be only one address (per shard) that can hold the role of creating a Meta ESDT, this contract was created so the multiple other contracts that need to create Locked MEX can just request them from only one place.

## Release and reproducible builds

Release builds are reproducible: every contract is built inside the Docker image pinned in [sc-reproducible-build.toml](sc-reproducible-build.toml), so anyone can rebuild the same commit and get the same codehash (this is also what the explorer uses to verify a contract). Only the contracts named for a release are built.

To release:

1. Create the release on GitHub, usually as a pre-release. This creates the tag.
2. In Actions → **Release** → **Run workflow** (from `main`), fill in:
   - `tag`: the release tag, e.g. `v3.5.0`;
   - `contracts`: the Cargo.toml package names, separated by spaces or commas, e.g. `pair router farm-staking`.
3. The workflow builds the tag and attaches the `.wasm`, `.abi.json`, `.source.json` and `<contract>.artifacts.json` files to the release. It also adds the Docker image, the commit and the blake2b codehashes to the release notes.

Running the workflow again for the same tag adds the new contracts to the release and keeps the codehashes already there. With `dry_run` it only builds (any branch, tag or commit) and keeps the files as a workflow artifact.

The workflow runs [reproducible-build.sh](reproducible-build.sh), which also works locally (needs Docker, `sc-meta` >= 0.66.1 and `python3`). It builds a clean export of a commit, so local changes and untracked files are left out:

```sh
./reproducible-build.sh --list                       # contract names
./reproducible-build.sh pair router                  # build HEAD
./reproducible-build.sh --ref v3.5.0 farm-staking    # check the codehashes of a release
```

The output goes to `target/reproducible-build/`, with the codehashes in `codehashes.md`.

To upgrade the toolchain, bump the image tag in [sc-reproducible-build.toml](sc-reproducible-build.toml), after checking the image's Rust and sc-meta versions against `rust-toolchain.toml` and the framework version.
