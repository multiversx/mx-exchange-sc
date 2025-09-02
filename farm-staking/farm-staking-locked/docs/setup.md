# Farm Staking setup steps

## Deployment

The init function takes the following arguments:
- farming_token_id - the farming token ID, which will also be the reward token
- division_safety_constant - used in some calculations for precision. I suggest something like 10^9.
- max_apr - a percentage of max APR, which will limit the user's rewards, with two decimals precision (i.e. 10_000 = 100%). Can be more than 100%.
- min_unbond_epochs - Number of epochs the user has to wait between unstake and unbond.
```
#[init]
fn init(
    &self,
    farming_token_id: TokenIdentifier,
    division_safety_constant: BigUint,
    max_apr: BigUint,
    min_unbond_epochs: u64,
)
```

## Additional config

### Setup farm token

You have to register/issue the farm token and set its local roles, which is the token used for positions in the staking contract. This is done through the following endpoints:

```
#[payable("EGLD")]
#[endpoint(registerFarmToken)]
fn register_farm_token(
    &self,
    #[payment_amount] register_cost: BigUint,
    token_display_name: ManagedBuffer,
    token_ticker: ManagedBuffer,
    num_decimals: usize,
)
```

For issue parameters format restrictions, take a look here: https://docs.multiversx.com/tokens/esdt-tokens#parameters-format

payment_amount should be `0.05 EGLD`.

```
#[endpoint(setLocalRolesFarmToken)]
fn set_local_roles_farm_token(&self)
```

### Set per block rewards

```
#[endpoint(setPerBlockRewardAmount)]
fn set_per_block_rewards(&self, per_block_amount: BigUint)
```

Keep in mind amount has to take into consideration the token's decimals. So if you have a token with 18 decimals, you have to pass 10^18 for "1".

### Add the reward tokens

```
#[payable("*")]
#[endpoint(topUpRewards)]
fn top_up_rewards(
    &self,
    #[payment_token] payment_token: TokenIdentifier,
    #[payment_amount] payment_amount: BigUint,
)
```

No args needed, you only need to pay the reward tokens. In the staking farm, rewards are not minted, but added by the owner.

### Final steps

First, you have to enable rewards generation:

```
#[endpoint(startProduceRewards)]
fn start_produce_rewards(&self)
```

Then, you have to the set the state to active:

```
#[endpoint]
fn resume(&self)
```
