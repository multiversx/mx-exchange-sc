multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::LockedAssetTokenAttributesEx;

#[derive(TypeAbi, TopEncode)]
pub struct CreateAndForwardEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    destination: ManagedAddress<M>,
    locked_asset_token_id: TokenIdentifier<M>,
    locked_asset_token_nonce: u64,
    locked_asset_token_amount: BigUint<M>,
    locked_assets_attributes: LockedAssetTokenAttributesEx<M>,
    start_epoch: u64,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[derive(TypeAbi, TopEncode)]
pub struct UnlockAssetsEvent<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    input_locked_assets_token_id: TokenIdentifier<M>,
    input_locked_assets_token_nonce: u64,
    input_locked_assets_token_amount: BigUint<M>,
    output_locked_assets_token_id: TokenIdentifier<M>,
    output_locked_assets_token_nonce: u64,
    output_locked_assets_token_amount: BigUint<M>,
    asset_token_id: TokenIdentifier<M>,
    asset_token_amount: BigUint<M>,
    input_assets_attributes: LockedAssetTokenAttributesEx<M>,
    output_assets_attributes: LockedAssetTokenAttributesEx<M>,
    block: u64,
    epoch: u64,
    timestamp: u64,
}

#[multiversx_sc::module]
pub trait EventsModule {
    fn emit_create_and_forward_event(
        self,
        caller: &ManagedAddress,
        destination: &ManagedAddress,
        locked_asset_token_id: TokenIdentifier,
        locked_asset_token_nonce: u64,
        locked_asset_token_amount: BigUint,
        locked_assets_attributes: LockedAssetTokenAttributesEx<Self::Api>,
        start_epoch: u64,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.create_and_forward_event(
            caller,
            destination,
            epoch,
            &CreateAndForwardEvent {
                caller: caller.clone(),
                destination: destination.clone(),
                locked_asset_token_id,
                locked_asset_token_nonce,
                locked_asset_token_amount,
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
        caller: &ManagedAddress,
        input_locked_assets_token_id: TokenIdentifier,
        input_locked_assets_token_nonce: u64,
        input_locked_assets_token_amount: BigUint,
        output_locked_assets_token_id: TokenIdentifier,
        output_locked_assets_token_nonce: u64,
        output_locked_assets_token_amount: BigUint,
        asset_token_id: TokenIdentifier,
        asset_token_amount: BigUint,
        input_assets_attributes: LockedAssetTokenAttributesEx<Self::Api>,
        output_assets_attributes: LockedAssetTokenAttributesEx<Self::Api>,
    ) {
        let epoch = self.blockchain().get_block_epoch();
        self.unlock_assets_event(
            caller,
            epoch,
            &UnlockAssetsEvent {
                caller: caller.clone(),
                input_locked_assets_token_id,
                input_locked_assets_token_nonce,
                input_locked_assets_token_amount,
                output_locked_assets_token_id,
                output_locked_assets_token_nonce,
                output_locked_assets_token_amount,
                asset_token_id,
                asset_token_amount,
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
        #[indexed] caller: &ManagedAddress,
        #[indexed] destination: &ManagedAddress,
        #[indexed] epoch: u64,
        swap_event: &CreateAndForwardEvent<Self::Api>,
    );

    #[event("unlock_assets")]
    fn unlock_assets_event(
        self,
        #[indexed] caller: &ManagedAddress,
        #[indexed] epoch: u64,
        swap_event: &UnlockAssetsEvent<Self::Api>,
    );
}
