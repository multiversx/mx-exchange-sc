#![no_std]
#![allow(deprecated)]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

type Nonce = u64;
type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type INCORRECTReturnType<ManagedTypeApi> = ManagedBuffer<ManagedTypeApi>;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
}

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub original_entering_epoch: u64,
    pub entering_epoch: u64,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

static ERROR_LEGACY_CONTRACT: &[u8] = b"This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.";

#[multiversx_sc::contract]
pub trait FarmV13CustomRewards {
    #[init]
    fn init(&self) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(addAddressToWhitelist)]
    fn add_address_to_whitelist(&self, _address: ManagedAddress) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        _amount: BigUint,
        _attributes: FarmTokenAttributes<Self::Api>,
    ) -> SCResult<BigUint> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> SCResult<ClaimRewardsResultType<Self::Api>> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(end_produce_rewards)]
    fn end_produce_rewards(&self) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getBlockForEndRewards)]
    fn block_for_end_rewards(&self) -> u64 {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getBurnGasLimit)]
    fn burn_gas_limit(&self) -> u64 {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getDivisionSafetyConstant)]
    fn division_safety_constant(&self) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getFarmTokenId)]
    fn farm_token_id(&self) -> TokenIdentifier {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getFarmTokenSupply)]
    fn farm_token_supply(&self) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getFarmingTokenId)]
    fn farming_token_id(&self) -> TokenIdentifier {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getLastErrorMessage)]
    fn last_error_message(&self) -> ManagedBuffer {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getLastRewardBlockNonce)]
    fn last_reward_block_nonce(&self) -> Nonce {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getMinimumFarmingEpoch)]
    fn minimum_farming_epochs(&self) -> u8 {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getOwner)]
    fn owner(&self) -> ManagedAddress {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getPenaltyPercent)]
    fn penalty_percent(&self) -> u64 {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getPerBlockRewardAmount)]
    fn per_block_reward_amount(&self) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getRewardPerShare)]
    fn reward_per_share(&self) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getRewardReserve)]
    fn reward_reserve(&self) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getRewardTokenId)]
    fn reward_token_id(&self) -> TokenIdentifier {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getState)]
    fn state(&self) -> State {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getTransferExecGasLimit)]
    fn transfer_exec_gas_limit(&self) -> u64 {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getWhitelist)]
    fn whitelist(&self) -> UnorderedSetMapper<ManagedAddress> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> SCResult<EsdtTokenPayment<Self::Api>> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(pause)]
    fn pause(&self) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("EGLD")]
    #[endpoint(registerFarmToken)]
    fn register_farm_token(
        &self,
        _token_display_name: ManagedBuffer,
        _token_ticker: ManagedBuffer,
        _num_decimals: usize,
    ) -> INCORRECTReturnType<Self::Api> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(removeAddressFromWhitelist)]
    fn remove_address_from_whitelist(&self, _address: ManagedAddress) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(resume)]
    fn resume(&self) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setBlockForEndRewards)]
    fn set_block_for_end_rewards(&self, _block_end: u64) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) -> INCORRECTReturnType<Self::Api> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_reward_amount(&self, _per_block_amount: BigUint) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(set_burn_gas_limit)]
    fn set_burn_gas_limit(&self, _gas_limit: u64) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(set_minimum_farming_epochs)]
    fn set_minimum_farming_epochs(&self, _epochs: u8) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(set_penalty_percent)]
    fn set_penalty_percent(&self, _percent: u64) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(set_transfer_exec_gas_limit)]
    fn set_transfer_exec_gas_limit(&self, _gas_limit: u64) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards(&self) -> SCResult<()> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }
}
