elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::locked_asset::LockedAssetTokenAttributes;
use common_structs::{FftTokenAmountPair, GenericTokenAmountPair};

#[derive(TopEncode)]
pub struct CreateAndForwardEvent<BigUint: BigUintApi> {
    caller: Address,
    destination: Address,
    locked_assets_token_amount: GenericTokenAmountPair<BigUint>,
    locked_assets_attributes: LockedAssetTokenAttributes,
    start_epoch: u64,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TopEncode)]
pub struct UnlockAssetsEvent<BigUint: BigUintApi> {
    caller: Address,
    input_locked_assets_token_amount: GenericTokenAmountPair<BigUint>,
    output_locked_assets_token_amount: GenericTokenAmountPair<BigUint>,
    assets_token_amount: FftTokenAmountPair<BigUint>,
    input_assets_attributes: LockedAssetTokenAttributes,
    output_assets_attributes: LockedAssetTokenAttributes,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[elrond_wasm::module]
pub trait EventsModule {
    fn emit_create_and_forward_event(
        self,
        caller: Address,
        destination: Address,
        locked_assets_token_amount: GenericTokenAmountPair<Self::BigUint>,
        locked_assets_attributes: LockedAssetTokenAttributes,
        start_epoch: u64,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.create_and_forward_event(
            caller.clone(),
            destination.clone(),
            epoch,
            CreateAndForwardEvent {
                caller,
                destination,
                locked_assets_token_amount,
                locked_assets_attributes,
                start_epoch,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    fn emit_unlock_assets_event(
        self,
        caller: Address,
        input_locked_assets_token_amount: GenericTokenAmountPair<Self::BigUint>,
        output_locked_assets_token_amount: GenericTokenAmountPair<Self::BigUint>,
        assets_token_amount: FftTokenAmountPair<Self::BigUint>,
        input_assets_attributes: LockedAssetTokenAttributes,
        output_assets_attributes: LockedAssetTokenAttributes,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.unlock_assets_event(
            caller.clone(),
            epoch,
            UnlockAssetsEvent {
                caller,
                input_locked_assets_token_amount,
                output_locked_assets_token_amount,
                assets_token_amount,
                input_assets_attributes,
                output_assets_attributes,
                block: self.blockchain().get_block_nonce(),
                epoch,
                timestamp: self.blockchain().get_block_timestamp(),
            },
        )
    }

    #[event("create_and_forward")]
    fn create_and_forward_event(
        self,
        #[indexed] caller: Address,
        #[indexed] destination: Address,
        #[indexed] epoch: u64,
        swap_event: CreateAndForwardEvent<Self::BigUint>,
    );

    #[event("unlock_assets")]
    fn unlock_assets_event(
        self,
        #[indexed] caller: Address,
        #[indexed] epoch: u64,
        swap_event: UnlockAssetsEvent<Self::BigUint>,
    );
}
