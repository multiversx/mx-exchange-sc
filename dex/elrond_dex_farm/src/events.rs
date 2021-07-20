elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{FftTokenAmountPair, GenericTokenAmountPair};

use crate::FarmTokenAttributes;

#[derive(TopEncode)]
pub struct EnterFarmEvent<BigUint: BigUintApi> {
    user_address: Address,
    farming_token_amount: FftTokenAmountPair<BigUint>,
    farming_reserves: BigUint,
    farm_token_amount: GenericTokenAmountPair<BigUint>,
    farm_supply: BigUint,
    reward_token_reserves: FftTokenAmountPair<BigUint>,
    farm_attributes: FarmTokenAttributes<BigUint>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct ExitFarmEvent<BigUint: BigUintApi> {
    user_address: Address,
    farming_token_amount: FftTokenAmountPair<BigUint>,
    farming_reserves: BigUint,
    farm_token_amount: GenericTokenAmountPair<BigUint>,
    farm_supply: BigUint,
    reward_token_amount: GenericTokenAmountPair<BigUint>,
    reward_reserves: BigUint,
    farm_attributes: FarmTokenAttributes<BigUint>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm_derive::module]
pub trait EventsModule {
    fn emit_enter_farm_event(
        &self,
        user_address: &Address,
        farming_token_amount: &FftTokenAmountPair<Self::BigUint>,
        farming_reserve: &Self::BigUint,
        farm_token_amount: &GenericTokenAmountPair<Self::BigUint>,
        farm_supply: &Self::BigUint,
        reward_token_reserves: &FftTokenAmountPair<Self::BigUint>,
        farm_attributes: &FarmTokenAttributes<Self::BigUint>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.enter_farm_event(
            user_address,
            farm_attributes.with_locked_rewards,
            epoch,
            EnterFarmEvent {
                user_address: user_address.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farming_reserves: farming_reserve.clone(),
                farm_token_amount: farm_token_amount.clone(),
                farm_supply: farm_supply.clone(),
                reward_token_reserves: reward_token_reserves.clone(),
                farm_attributes: farm_attributes.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_exit_farm_event(
        &self,
        user_address: &Address,
        farming_token_amount: &FftTokenAmountPair<Self::BigUint>,
        farming_reserves: &Self::BigUint,
        farm_token_amount: &GenericTokenAmountPair<Self::BigUint>,
        farm_supply: &Self::BigUint,
        reward_token_amount: &GenericTokenAmountPair<Self::BigUint>,
        reward_reserves: &Self::BigUint,
        farm_attributes: &FarmTokenAttributes<Self::BigUint>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.exit_farm_event(
            user_address,
            farm_attributes.with_locked_rewards,
            epoch,
            ExitFarmEvent {
                user_address: user_address.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farming_reserves: farming_reserves.clone(),
                farm_token_amount: farm_token_amount.clone(),
                farm_supply: farm_supply.clone(),
                reward_token_amount: reward_token_amount.clone(),
                reward_reserves: reward_reserves.clone(),
                farm_attributes: farm_attributes.clone(),
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    #[event("enter_farm")]
    fn enter_farm_event(
        &self,
        #[indexed] user_address: &Address,
        #[indexed] with_locked_rewards: bool,
        #[indexed] epoch: u64,
        enter_farm_event: EnterFarmEvent<Self::BigUint>,
    );

    #[event("exit_farm")]
    fn exit_farm_event(
        &self,
        #[indexed] user_address: &Address,
        #[indexed] with_locked_rewards: bool,
        #[indexed] epoch: u64,
        exit_farm_event: ExitFarmEvent<Self::BigUint>,
    );
}
