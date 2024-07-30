#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

type Nonce = u64;
type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

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

#[multiversx_sc::contract]
pub trait FarmV13LockedRewards {
    #[init]
    fn init(&self) {}

    #[only_owner]
    #[endpoint(addAddressToWhitelist)]
    fn add_address_to_whitelist(&self, _address: ManagedAddress) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        _amount: BigUint,
        _attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ClaimRewardsResultType<Self::Api> {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[only_owner]
    #[payable("*")]
    #[endpoint(depositRewards)]
    fn deposit_rewards(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(end_produce_rewards)]
    fn end_produce_rewards(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> EnterFarmResultType<Self::Api> {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getBlockForEndRewards)]
    fn block_for_end_rewards(&self) -> u64 {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getBurnGasLimit)]
    fn burn_gas_limit(&self) -> u64 {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getDivisionSafetyConstant)]
    fn division_safety_constant(&self) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getFarmTokenId)]
    fn farm_token_id(&self) -> TokenIdentifier {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getFarmTokenSupply)]
    fn farm_token_supply(&self) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getFarmingTokenId)]
    fn farming_token_id(&self) -> TokenIdentifier {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getLastErrorMessage)]
    fn last_error_message(&self) -> ManagedBuffer {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getLastRewardBlockNonce)]
    fn last_reward_block_nonce(&self) -> Nonce {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getMinimumFarmingEpoch)]
    fn minimum_farming_epochs(&self) -> u8 {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getOwner)]
    fn owner(&self) -> ManagedAddress {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getPenaltyPercent)]
    fn penalty_percent(&self) -> u64 {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getPerBlockRewardAmount)]
    fn per_block_reward_amount(&self) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getRewardPerShare)]
    fn reward_per_share(&self) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getRewardReserve)]
    fn reward_reserve(&self) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getRewardTokenId)]
    fn reward_token_id(&self) -> TokenIdentifier {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getState)]
    fn state(&self) -> State {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getTransferExecGasLimit)]
    fn transfer_exec_gas_limit(&self) -> u64 {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getWhitelist)]
    fn whitelist(&self) -> UnorderedSetMapper<ManagedAddress> {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> EsdtTokenPayment {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(pause)]
    fn pause(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("EGLD")]
    #[endpoint(registerFarmToken)]
    fn register_farm_token(
        &self,
        _token_display_name: ManagedBuffer,
        _token_ticker: ManagedBuffer,
        _num_decimals: usize,
    ) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[only_owner]
    #[endpoint(removeAddressFromWhitelist)]
    fn remove_address_from_whitelist(&self, _address: ManagedAddress) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(resume)]
    fn resume(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[only_owner]
    #[endpoint(setBlockForEndRewards)]
    fn set_block_for_end_rewards(&self, _block_end: u64) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_reward_amount(&self, _per_block_amount: BigUint) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(set_burn_gas_limit)]
    fn set_burn_gas_limit(&self, _gas_limit: u64) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(set_minimum_farming_epochs)]
    fn set_minimum_farming_epochs(&self, _epochs: u8) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(set_penalty_percent)]
    fn set_penalty_percent(&self, _percent: u64) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(set_transfer_exec_gas_limit)]
    fn set_transfer_exec_gas_limit(&self, _gas_limit: u64) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }
}
