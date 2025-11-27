#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

type Nonce = u64;
pub type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq)]
pub enum State {
    Inactive,
    Active,
}

#[type_abi]
#[derive(
    ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq, Debug,
)]
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[type_abi]
#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone)]
pub struct FarmTokenAttributesV1_2<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub apr_multiplier: u8,
    pub with_locked_rewards: bool,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq)]
pub enum FarmMigrationRole {
    Old,
    New,
    NewWithLock,
}

#[type_abi]
#[derive(TopEncode, TopDecode)]
pub struct FarmMigrationConfig<M: ManagedTypeApi> {
    migration_role: FarmMigrationRole,
    old_farm_address: ManagedAddress<M>,
    old_farm_token_id: TokenIdentifier<M>,
}

static ERROR_LEGACY_CONTRACT: &[u8] = b"This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.";

#[multiversx_sc::contract]
pub trait FarmV13LockedRewards {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> EsdtTokenPayment<Self::Api> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        _amount: BigUint,
        _attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn end_produce_rewards(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards(&self, _per_block_amount: BigUint) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn set_penalty_percent(&self, _percent: u64) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn set_minimum_farming_epochs(&self, _epochs: u8) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn set_transfer_exec_gas_limit(&self, _gas_limit: u64) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn set_burn_gas_limit(&self, _gas_limit: u64) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(setRpsAndStartRewards)]
    fn set_rps_and_start_rewards(&self, _rps: BigUint) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn pause(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint]
    fn resume(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_as_owner(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setFarmTokenSupply)]
    fn set_farm_token_supply(&self, _supply: BigUint) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setFarmMigrationConfig)]
    fn set_farm_migration_config(
        &self,
        _old_farm_address: ManagedAddress,
        _old_farm_token_id: TokenIdentifier,
        _new_farm_address: ManagedAddress,
        _new_farm_with_lock_address: ManagedAddress,
    ) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerFarmToken)]
    fn register_farm_token(
        &self,
        _token_display_name: ManagedBuffer,
        _token_ticker: ManagedBuffer,
        _num_decimals: usize,
    ) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint(migrateFromV1_2Farm)]
    fn migrate_from_v1_2_farm(
        &self,
        _old_attrs: FarmTokenAttributesV1_2<Self::Api>,
        _orig_caller: ManagedAddress,
    ) -> EsdtTokenPayment<Self::Api> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getFarmMigrationConfiguration)]
    #[storage_mapper("farm_migration_config")]
    fn farm_migration_config(&self) -> SingleValueMapper<FarmMigrationConfig<Self::Api>>;

    #[view(getFarmTokenSupply)]
    #[storage_mapper("farm_token_supply")]
    fn farm_token_supply(&self) -> SingleValueMapper<BigUint>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

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
    fn minimum_farming_epochs(&self) -> SingleValueMapper<u8>;

    #[view(getPerBlockRewardAmount)]
    #[storage_mapper("per_block_reward_amount")]
    fn per_block_reward_amount(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("produce_rewards_enabled")]
    fn produce_rewards_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getLastRewardBlockNonce)]
    #[storage_mapper("last_reward_block_nonce")]
    fn last_reward_block_nonce(&self) -> SingleValueMapper<Nonce>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farm_token_id")]
    fn farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

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

    #[view(getRewardPerShare)]
    #[storage_mapper("reward_per_share")]
    fn reward_per_share(&self) -> SingleValueMapper<BigUint>;

    #[view(getRewardReserve)]
    #[storage_mapper("reward_reserve")]
    fn reward_reserve(&self) -> SingleValueMapper<BigUint>;

    #[view(getTransferExecGasLimit)]
    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<u64>;
}
