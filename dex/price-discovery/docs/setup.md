# Setup docs

The contract defines two tokens:
- launched_token_id - the token identifier of the newly launched token on the XExchange.
- accepted_token_id - an already established token, that will be used to determine the price of the launched token

Additionally, a min price is also defined by the following arguments:
- launched_token_decimals - the number of decimals for the launched token. Most tokens have 18 decimals.
- a min_launched_token_price is the minimum price of the launched token, in accepted tokens. This value has to be denominated according to the ACCEPTED token decimals. For example, if we have launched token with 18 decimals, and accepted token with 6 decimals, and we want a 1:1 ratio, the min_price argument should be 1_000_000.

Next we define the length of the phases. Over the start-end period, we define multiple phases, 
in which interactions with the Price Discovery SC will impose some restrictions:
    1) No restrictions. Anyone can deposit/withdraw any amount
    2) Deposits are unrestricted, withdrawals come with a linear increasing penalty
    3) Deposits are not allowed, withdrawals come with a fixed penalty
    4) Neither deposits nor withdrawals are allowed. This is when the LP is created.

- start_block - phase 1 start block
- for no_limit_phase_duration_blocks - phase 1 duration
- linear_penalty_phase_duration_blocks - phase 2 duration
- fixed_penalty_phase_duration_blocks - phase 3 duration
- unlock_epoch - the unlock epoch for the redeemed tokens. Users will receive locked tokens and can unlock them at the specified epoch through an external contract
- penalty_min_percentage, penalty_max_percentage - the minimum and maximum percentage for Phase 2).
    The percentage increases linearly between phase 2's start and end
- fixed_penalty_percentage - The penalty percentage for phase 3.

- locking_sc_address - additionally, as suggested by the `unlock_epoch` argument, we need the address of the locking SC. This contract's source code can be found in the `locked_asset/simple-lock` folder.

```rust
#[init]
fn init(
    &self,
    launched_token_id: TokenIdentifier,
    accepted_token_id: TokenIdentifier,
    launched_token_decimals: u32,
    min_launched_token_price: BigUint,
    start_block: u64,
    no_limit_phase_duration_blocks: u64,
    linear_penalty_phase_duration_blocks: u64,
    fixed_penalty_phase_duration_blocks: u64,
    unlock_epoch: u64,
    penalty_min_percentage: BigUint,
    penalty_max_percentage: BigUint,
    fixed_penalty_percentage: BigUint,
    locking_sc_address: ManagedAddress,
)
```

After deployment, the SC requires the `redeem_token` to be issued and have its roles set. This is done through the `issue_redeem_token` endpoint:
```
#[only_owner]
#[payable("EGLD")]
#[endpoint(issueRedeemToken)]
fn issue_redeem_token(
    &self,
    token_name: ManagedBuffer,
    token_ticker: ManagedBuffer,
    nr_decimals: usize,
)
```

The redeem token is a meta ESDT token that the users receive on deposits. Those can then be used to withdraw the initial tokens (or part of them, as per phase restrictions). We only use two nonces:
- nonce 1 for launched tokens
- nonce 2 for accepted tokens

In the issue callback, one of each of those tokens is created, so that the SC can afterwards use NFTAddQuantity. These tokens have no additional attributes.
