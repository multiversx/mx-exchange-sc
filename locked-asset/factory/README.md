# Locked Asset Factory Smart Contract

This document presents how one can deploy and configure a Locked Asset Factory Contract.
The bigger picture about what a Locked Asset Factory Contract can do can be found in the Repository's Root Readme.

## Deployment

The Locked Asset Factory contract can be deployed using `erdpy` and using the interaction snippets.

The init parameters are:

- asset_token_id. The TokenId of the asset that a locked asset represents. In case of Maiar Exchange it will be MEX.

- default_unlock_period. A vector of unlock milestones. This represents a period since each epoch in the vector will be added with a starting epoch.

The Contract requires LocalMint and LocalBurn for asset token.

## Creating and Forwarding SFTs

Before creating LockedAssetLockens, the owner has to issue those tokens using `issueLockedAssetToken` and after this he also has to give the NftCreate, NftAddQuantity and NftBurn roles to the contract unsing `setLocalRolesLockedAssetToken`.

The Contract has an endpoint `createAndForward` that can be called in order to request an amount of Locked MEX. Only those addresses in the `whitelisted_contracts` set can call this endpoint. This whitelist can be configured by the admin using `whitelist` and `removeWhitelist` endpoints.

## Unlocking MEX

A user that has Locked MEX can unlock it and can receive the Locked MEX "remaining" and the unlocked MEX amount. The newly created Locked MEX will have its unlock milestones re-calculated such that the percents unlocking schedule will be updated to the new locked amount. For example: if default_unlock_period is `0x000000000000000232`, `0x000000000000000432` it would mean that after `0000000000000002` epochs, should unlock `32`.to_dec() (`50`) percent of the amount. After the first unlock at epoch 3 let's say, the next unlock milestone will be recalculated as `0x000000000000000464`. Notice the `50%` become `100%`.
