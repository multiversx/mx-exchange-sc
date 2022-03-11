elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::config;
use crate::errors::*;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Debug)]
pub struct LockedAssetAttributes<M: ManagedTypeApi> {
    pub unlock_epoch: u64,
    pub amount: BigUint<M>,
}

#[elrond_wasm::module]
pub trait LockedAsset: config::ConfigModule + token_send::TokenSendModule {
    #[endpoint(setLockedAssetTokenIdFirst)]
    fn set_locked_asset_token_id_first(&self, token_id: TokenIdentifier) {
        self.require_permissions();
        self.locked_asset_token_id_first().set(token_id);
    }

    #[endpoint(setLockedAssetTokenIdSecond)]
    fn set_locked_asset_token_id_second(&self, token_id: TokenIdentifier) {
        self.require_permissions();
        self.locked_asset_token_id_second().set(token_id);
    }

    #[endpoint(setLockingPeriodInEpochs)]
    fn set_locking_period_in_epochs(&self, num_epochs: u64) {
        self.require_permissions();
        self.locking_period_in_epochs().set(num_epochs);
    }

    #[endpoint(setLockedAssetGenerateEpochLimit)]
    fn set_locked_asset_generate_epoch_limit(&self, epoch: u64) {
        self.require_permissions();
        self.locked_asset_generate_epoch_limit().set(epoch);
    }

    #[payable("*")]
    #[endpoint(unlockAssets)]
    fn unlock_assets(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_nonce] payment_nonce: u64,
    ) -> EsdtTokenPayment<Self::Api> {
        let token_to_send_back = if payment_token == self.locked_asset_token_id_first().get() {
            self.first_token_id().get()
        } else if payment_token == self.locked_asset_token_id_second().get() {
            self.second_token_id().get()
        } else {
            sc_panic!(ERROR_UNKNOWN_TOKEN);
        };

        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            &payment_token,
            payment_nonce,
        );
        let attr: LockedAssetAttributes<Self::Api> = token_info.decode_attributes();

        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attr.unlock_epoch,
            ERROR_UNLOCK_CALLED_TOO_EARLY
        );

        self.send().direct(
            &self.blockchain().get_caller(),
            &token_to_send_back,
            0,
            &attr.amount,
            &[],
        );

        self.send()
            .esdt_local_burn(&payment_token, payment_nonce, &BigUint::from(1u64));

        EsdtTokenPayment::new(token_to_send_back, 0, attr.amount)
    }

    fn should_generate_locked_asset(&self, is_first_token: bool) -> bool {
        let is_locked_asset_empty = if is_first_token {
            self.locked_asset_token_id_first().is_empty()
        } else {
            self.locked_asset_token_id_second().is_empty()
        };

        if is_locked_asset_empty {
            return false;
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let epoch_limit = self.locked_asset_generate_epoch_limit().get();
        let locking_period = self.locking_period_in_epochs().get();

        current_epoch <= epoch_limit && locking_period != 0
    }

    fn generate_locked_asset(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        let locked_asset_id = if token_id == &self.first_token_id().get() {
            self.locked_asset_token_id_first().get()
        } else {
            self.locked_asset_token_id_second().get()
        };

        let current_epoch = self.blockchain().get_block_epoch();
        let locking_period = self.locking_period_in_epochs().get();
        let attr = LockedAssetAttributes {
            amount: amount.clone(),
            unlock_epoch: current_epoch + locking_period,
        };

        let nonce = self.send().esdt_nft_create(
            &locked_asset_id,
            &BigUint::from(1u64),
            &ManagedBuffer::new(),
            &BigUint::zero(),
            &ManagedBuffer::new(),
            &attr,
            &ManagedVec::new(),
        );

        EsdtTokenPayment::new(locked_asset_id, nonce, BigUint::from(1u64))
    }

    #[view(getLockedAssetGenerateEpochLimit)]
    #[storage_mapper("locked_asset_generate_epoch_limit")]
    fn locked_asset_generate_epoch_limit(&self) -> SingleValueMapper<u64>;

    #[view(getLockingPeriodInEpochs)]
    #[storage_mapper("locking_period_in_epochs")]
    fn locking_period_in_epochs(&self) -> SingleValueMapper<u64>;

    #[view(getLockedAssetTokenIdFirst)]
    #[storage_mapper("locked_asset_token_id_first")]
    fn locked_asset_token_id_first(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLockedAssetTokenIdSecond)]
    #[storage_mapper("locked_asset_token_id_second")]
    fn locked_asset_token_id_second(&self) -> SingleValueMapper<TokenIdentifier>;
}
