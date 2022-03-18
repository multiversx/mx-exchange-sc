# Setup docs

The contract defines three tokens:
- launched_token_id - the token identifier of the newly launched token on the Elrond DEX
- accepted_token_id - an already established token, that will be used to determine the price of the launched token
- extra_rewards_token_id - the token identifier of some extra rewards, which will be recevied by those who contributed to the pool. These can be deposited by anyone.
- a min_launched_token_price is the minimum price of the launched token, in accepted tokens. The precision is defined by the MIN_PRICE_PRECISION constant.A min price of "MIN_PRICE_PRECISION" means the minimum price of the launched token cannot go below 1 accepted token (i.e. minimum 1:1 ratio must be maintained)

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
- unbond_period_epochs - LP tokens received from the LP pool are locked for this number of epochs
    before withdrawal is allowed
- penalty_min_percentage, penalty_max_percentage - the minimum and maximum percentage for Phase 2).
    The percentage increases linearly between phase 2's start and end
- fixed_penalty_percentage - The penalty percentage for phase 3.

```rust
#[init]
fn init(
    &self,
    launched_token_id: TokenIdentifier,
    accepted_token_id: TokenIdentifier,
    extra_rewards_token_id: TokenIdentifier,
    min_launched_token_price: BigUint,
    start_block: u64,
    no_limit_phase_duration_blocks: u64,
    linear_penalty_phase_duration_blocks: u64,
    fixed_penalty_phase_duration_blocks: u64,
    unbond_period_epochs: u64,
    penalty_min_percentage: BigUint,
    penalty_max_percentage: BigUint,
    fixed_penalty_percentage: BigUint,
)
```

Before the deposits/withdrawals can begin, the DEX pair SC address must be set through `set_pair_address`.
