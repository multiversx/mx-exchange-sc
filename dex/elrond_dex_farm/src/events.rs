elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use dex_common::{FftTokenAmountPair, GenericEsdtAmountPair};

use crate::FarmTokenAttributes;

#[derive(TopEncode)]
pub struct EnterFarmEvent<BigUint: BigUintApi> {
    sc_address: Address,
    user_address: Address,
    farming_token_amount: FftTokenAmountPair<BigUint>,
    farm_token_amount: GenericEsdtAmountPair<BigUint>,
    farm_attributes: FarmTokenAttributes<BigUint>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct ExitFarmEvent<BigUint: BigUintApi> {
    sc_address: Address,
    user_address: Address,
    farming_token_amount: FftTokenAmountPair<BigUint>,
    farm_token_amount: GenericEsdtAmountPair<BigUint>,
    reward_token_amount: FftTokenAmountPair<BigUint>,
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
        farm_token_amount: &GenericEsdtAmountPair<Self::BigUint>,
        farm_attributes: &FarmTokenAttributes<Self::BigUint>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.enter_farm_event(
            farm_attributes.with_locked_rewards,
            epoch,
            EnterFarmEvent {
                sc_address: self.blockchain().get_sc_address(),
                user_address: user_address.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farm_token_amount: farm_token_amount.clone(),
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
        farm_token_amount: &GenericEsdtAmountPair<Self::BigUint>,
        reward_token_amount: &FftTokenAmountPair<Self::BigUint>,
        farm_attributes: &FarmTokenAttributes<Self::BigUint>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.exit_farm_event(
            farm_attributes.with_locked_rewards,
            epoch,
            ExitFarmEvent {
                sc_address: self.blockchain().get_sc_address(),
                user_address: user_address.clone(),
                farming_token_amount: farming_token_amount.clone(),
                farm_token_amount: farm_token_amount.clone(),
                reward_token_amount: reward_token_amount.clone(),
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
        #[indexed] with_locked_rewards: bool,
        #[indexed] epoch: u64,
        enter_farm_event: EnterFarmEvent<Self::BigUint>,
    );

    #[event("exit_farm")]
    fn exit_farm_event(
        &self,
        #[indexed] with_locked_rewards: bool,
        #[indexed] epoch: u64,
        exit_farm_event: ExitFarmEvent<Self::BigUint>,
    );
}
