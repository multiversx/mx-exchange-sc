# Router Smart Contract

This document presents how one can deploy and configure a Router contract.
The bigger picture about what a Router contract can do can be found in the Repository's Root Readme.

## Deployment

The Router can be deployed using `erdpy` and using interaction snippets. It takes no arguments. Pair creation is by default disabled for normal users.

## LP Tokens

The Router Contract is the owner of all LP Tokens in Maiar exchange. Hence, it was to accord LocalRoles to every pair and/or farm contract that Mint and/or Burn those Tokens.

## Pair Code Construction

The contract code can be constructed in at least 3 transactions. This is because if the pair code is too big, it cannot be uploaded in one single transaction. If the code itself needs at least two transactions, other two methods (start, end) were created to limit this whole process as being an atomic one.

The admin should do the following for constructing the pair contract code:
`startPairCodeConstruction`, `appendPairCode` (can be multiple calls), `endPairCodeConstruction`.

## Pair Contract Deployment

The basic deployment scenario of a Pair Contract by a user (assuming this option is enabled) is done with 3 transactions: `createPair`, `issueLpToken`, `setLocalRoles`. Issuing an LP Token for a specific pair can be done only by the initiator of the pair (the same user that called createPair) in the first 5 minutes. If that user did not issue an LP Token, any user can continue the creating process.
