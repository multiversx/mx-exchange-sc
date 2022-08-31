#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_errors::*;

use common_structs::{Epoch, Nonce};
use pausable::State;

pub const MAX_PERCENT: u64 = 10_000;
pub const DEFAULT_PENALTY_PERCENT: u64 = 100;
pub const DEFAULT_MINUMUM_FARMING_EPOCHS: u64 = 3;
pub const DEFAULT_BURN_GAS_LIMIT: u64 = 50_000_000;
pub const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;
pub const MAX_MINIMUM_FARMING_EPOCHS: u64 = 30;

#[elrond_wasm::module]
pub trait ConfigModule:
    token_send::TokenSendModule + pausable::PausableModule + permissions_module::PermissionsModule
{
    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active
    }

    #[only_owner]
    #[endpoint]
    fn set_penalty_percent(&self, percent: u64) {
        require!(percent < MAX_PERCENT, ERROR_PARAMETERS);
        self.penalty_percent().set(percent);
    }

    #[endpoint]
    fn set_minimum_farming_epochs(&self, epochs: Epoch) {
        self.require_caller_has_admin_permissions();
        require!(epochs <= MAX_MINIMUM_FARMING_EPOCHS, ERROR_PARAMETERS);

        self.minimum_farming_epochs().set(epochs);
    }

    #[only_owner]
    #[endpoint]
    fn set_burn_gas_limit(&self, gas_limit: u64) {
        self.burn_gas_limit().set(gas_limit);
    }

    #[view(getFarmTokenSupply)]
    #[storage_mapper("farm_token_supply")]
    fn farm_token_supply(&self) -> SingleValueMapper<BigUint>;

    #[view(getFarmingTokenId)]
    #[storage_mapper("farming_token_id")]
    fn farming_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getRewardTokenId)]
    #[storage_mapper("reward_token_id")]
    fn reward_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getPenaltyPercent)]
    #[storage_mapper("penalty_percent")]
    fn penalty_percent(&self) -> SingleValueMapper<u64>;

    #[view(getMinimumFarmingEpoch)]
    #[storage_mapper("minimum_farming_epochs")]
    fn minimum_farming_epochs(&self) -> SingleValueMapper<Epoch>;

    #[view(getPerBlockRewardAmount)]
    #[storage_mapper("per_block_reward_amount")]
    fn per_block_reward_amount(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("produce_rewards_enabled")]
    fn produce_rewards_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getLastRewardBlockNonce)]
    #[storage_mapper("last_reward_block_nonce")]
    fn last_reward_block_nonce(&self) -> SingleValueMapper<Nonce>;

    #[view(getDivisionSafetyConstant)]
    #[storage_mapper("division_safety_constant")]
    fn division_safety_constant(&self) -> SingleValueMapper<BigUint>;

    #[view(getPairContractManagedAddress)]
    #[storage_mapper("pair_contract_address")]
    fn pair_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getBurnGasLimit)]
    #[storage_mapper("burn_gas_limit")]
    fn burn_gas_limit(&self) -> SingleValueMapper<u64>;

    #[view(getLockedAssetFactoryManagedAddress)]
    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;
}
