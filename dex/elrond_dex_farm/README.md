# Farm Smart Contract

This document presents how one can deploy and configure a Farm contract.
The bigger picture about what a Farm contract can do can be found in the Repository's Root Readme.

## Deployment

The Farm Contract can be deployed using `erdpy` and using the interaction snippets.

The init parameters are:

- router_address. The address of the router. This may seem a bit useless because the router cannot configure a Farm contract, but we'll keep the address there in case it will be needed.

- reward_token_id. The reward token. In Maiar Exchange, this will usually be MEX.

- farming_token_id. The token used when entering a farm aka. the token that one farms with.

- Locked_asset_factory_address. The address of the locked asset factory contract. This is needed because One Smart Contract only (by protocol design) may have the Create role for a Specific SFT. Since more than one contract needs to be able to deliver Locked Mex, we designed one that does just that and accepts creation requests from other contracts.

- Division safety constant. To avoid having small numbers divided by big numbers, we believe that the approach of multiplying the small number with a specific constant and then divided by a big number is a good approach. Its value may depend on the magnitude of Farming Tokens and Reward Tokens. In Maiar Exchange, the common value for this constant is 1e12.

## Issuance of Farm Token

Issuance of Farm Token is can be done via `issueFarmToken` endpoint. Setting local roles can be done via `setLocalRolesFarmToken`. Those two calls are mandatory for a Farm to work.

## Producing rewards

In order for a Farm to produce rewards, the farm should be granted the LocalMint for Reward Tokens. After doing that, the admin should make these calls: `setPerBlockRewardAmount` and `start_produce_rewards`. After this, the contract will produce rewards on every block. A subtle thing here is that the contract won't actually produce the rewards on every block since it cannot have a timer or anything like this inside it. Instead, any action like `EnterFarm`, `ExitFarm`, `ClaimRewards`, `setPerBlockRewardAmount`, `stop_produce_rewards` will trigger minting of rewards.
