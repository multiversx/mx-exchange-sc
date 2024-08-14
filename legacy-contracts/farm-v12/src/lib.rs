#![no_std]
#![allow(deprecated)]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

type Nonce = u64;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type INCORRECTReturnType<ManagedTypeApi> = ManagedBuffer<ManagedTypeApi>;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
    Migrate,
}

#[multiversx_sc::contract]
pub trait FarmV12 {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint(acceptFee)]
    fn accept_fee(&self) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        _amount: BigUint,
        _attributes_raw: ManagedBuffer,
    ) -> SCResult<BigUint> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(end_produce_rewards_as_owner)]
    fn end_produce_rewards_as_owner(&self) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        _opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> SCResult<ExitFarmResultType<Self::Api>> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getBurnedTokenAmount)]
    fn burned_tokens(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getCurrentBlockFee)]
    fn current_block_fee_storage(&self) -> Option<(Nonce, BigUint)> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getDivisionSafetyConstant)]
    fn division_safety_constant(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getFarmTokenId)]
    fn farm_token_id(&self) -> TokenIdentifier {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getFarmTokenSupply)]
    fn get_farm_token_supply(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getFarmingTokenId)]
    fn farming_token_id(&self) -> TokenIdentifier {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getFarmingTokenReserve)]
    fn farming_token_reserve(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getLastErrorMessage)]
    fn last_error_message(&self) -> ManagedBuffer {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getLastRewardBlockNonce)]
    fn last_reward_block_nonce(&self) -> Nonce {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getLockedAssetFactoryManagedAddress)]
    fn locked_asset_factory_address(&self) -> ManagedAddress {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getLockedRewardAprMuliplier)]
    fn locked_rewards_apr_multiplier(&self) -> u8 {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getMinimumFarmingEpoch)]
    fn minimum_farming_epoch(&self) -> u8 {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getOwner)]
    fn owner(&self) -> ManagedAddress {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getPairContractManagedAddress)]
    fn pair_contract_address(&self) -> ManagedAddress {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getPenaltyPercent)]
    fn penalty_percent(&self) -> u64 {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getPerBlockRewardAmount)]
    fn per_block_reward_amount(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getRewardPerShare)]
    fn reward_per_share(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getRewardReserve)]
    fn reward_reserve(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getRewardTokenId)]
    fn reward_token_id(&self) -> TokenIdentifier {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getRouterManagedAddress)]
    fn router_address(&self) -> ManagedAddress {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getState)]
    fn state(&self) -> State {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getTransferExecGasLimit)]
    fn transfer_exec_gas_limit(&self) -> u64 {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getUndistributedFees)]
    fn undistributed_fee_storage(&self) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(pause)]
    fn pause(&self) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(resume)]
    fn resume(&self) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_reward_amount(&self, _per_block_amount: BigUint) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[only_owner]
    #[endpoint(setTransferRoleFarmToken)]
    fn set_transfer_role_farm_token(
        &self,
        _opt_address: OptionalValue<ManagedAddress>,
    ) -> INCORRECTReturnType<Self::Api> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(set_locked_rewards_apr_multiplier)]
    fn set_locked_rewards_apr_multiplier(&self, _muliplier: u8) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(set_minimum_farming_epochs)]
    fn set_minimum_farming_epochs(&self, _epochs: u8) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(set_penalty_percent)]
    fn set_penalty_percent(&self, _percent: u64) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(set_transfer_exec_gas_limit)]
    fn set_transfer_exec_gas_limit(&self, _gas_limit: u64) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint(start_produce_rewards)]
    fn start_produce_rewards(&self) -> SCResult<()> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }
}
