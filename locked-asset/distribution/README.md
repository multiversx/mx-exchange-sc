# Distribution Smart Contract

This document presents how one can deploy and configure a Distribution Contract.
The bigger picture about what a Distribution Contract can do can be found in the Repository's Root Readme.

## Deployment

The Distribution contract can be deployed using `erdpy` and using the interaction snippets.

The init parameters are:

- asset_token_id. The TokenId of the asset that a locked asset represents. In case of Maiar Exchange it will be MEX.

- locked_asset_factory_address. ManagedAddress of the locked asset factory which is used to request creation of Locked MEX tokens.

## Configuring Distribution

The basic workflow for an admin to set the community distribution is the following:

- `startGlobalOperation`. Which ensures no user activity is done until the end of the Global Operation

- `setCommunityDistribution`. Sets the total amount of Locked MEX that will be distributed to the community. Also sets the spread epoch, meaning the epoch after which a user can claim his tokens.

- `setPerUserDistributedLockedAssets`. Sets per user amount of Locked MEX to distribute.

- `endGlobalOperation`

The admin also has a set of endpoint and view to check and undo his actions if needed.

## Claiming distributed Locked MEX

The user has a view which he can use to query how many tokens he will receive by calling `claimLockedAssets`. A user cannot collect all the distributed series of tokens, instead he should be active, because he can only collect the last 4 series. If a user should have received 5 series of rewards, by claiming his locked assets, we will receive tokens only for the latest 4 series.
