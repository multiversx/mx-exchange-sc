elrond_wasm::imports!();

use common_structs::{Epoch, Nonce};

#[elrond_wasm::module]
pub trait SimpleLockMigrationModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_whitelist::TokenWhitelistModule
    + crate::util::UtilModule
    + crate::energy::EnergyModule
    + elrond_wasm_modules::pause::PauseModule
{
    /// Sets the LOCKED token ID. This is required, since we use an already existing token,
    /// instead of issue-ing a new one.
    ///
    /// The SC must already have the following roles before this function can be called:
    /// - NFTCReate
    /// - NFTAddQuantity
    /// - NFTBurn
    /// - TransferRole
    #[only_owner]
    #[endpoint(setLockedTokenId)]
    fn set_locked_token_id(&self, token_id: TokenIdentifier) {
        self.require_paused();
        self.require_valid_token_id(&token_id);
        self.require_has_roles_for_locked_token(&token_id);

        let own_sc_address = self.blockchain().get_sc_address();
        let last_old_token_nonce = self
            .blockchain()
            .get_current_esdt_nft_nonce(&own_sc_address, &token_id);
        let new_token_start_nonce = last_old_token_nonce + 1;

        self.new_token_start_nonce().set(new_token_start_nonce);
        self.locked_token().set_token_id(&token_id);
    }

    fn require_has_roles_for_locked_token(&self, token_id: &TokenIdentifier) {
        let actual_roles = self.blockchain().get_esdt_local_roles(token_id);
        let required_roles = EsdtLocalRoleFlags::NFT_CREATE
            | EsdtLocalRoleFlags::NFT_ADD_QUANTITY
            | EsdtLocalRoleFlags::NFT_BURN
            | EsdtLocalRoleFlags::TRANSFER;
        require!(
            actual_roles.contains(required_roles),
            "SC does not have ESDT transfer role for {}",
            token_id
        );
    }

    /// Sets the address for the contract which is expected to perform the migration
    #[only_owner]
    #[endpoint(setOldLockedAssetFactoryAddress)]
    fn set_old_locked_asset_factory_address(&self, old_sc_address: ManagedAddress) {
        self.require_paused();
        require!(
            self.old_locked_asset_factory_address().is_empty(),
            "Migration already started"
        );
        require!(
            self.blockchain().is_smart_contract(&old_sc_address),
            "Invalid SC address"
        );

        self.old_locked_asset_factory_address().set(&old_sc_address);
    }

    /// Converts old tokens from the locked asset factory into the new version.
    /// Additionally, it also updates the user's energy accordingly.
    ///
    /// This endpoint can only be called through the "migrateToNewFactory" endpoint
    /// from locked asset factory, and may not be called directly
    ///
    /// Expect input payment: total base assets locked under the given positions
    ///
    /// Expected arguments:
    /// - original_caller: the caller from the "migrateToNewFactory" call
    /// - amount_unlock_epoch_pairs: constructed from the original attributes
    /// by locked asset factory. Each milestone entry will generate a different token
    ///
    /// Output payments: New version of the locked tokens
    #[payable("*")]
    #[endpoint(acceptMigratedTokens)]
    fn accept_migrated_tokens(
        &self,
        original_caller: ManagedAddress,
        amount_unlock_epoch_pairs: MultiValueEncoded<MultiValue2<BigUint, Epoch>>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        self.require_not_paused();

        let caller = self.blockchain().get_caller();
        self.require_old_sc_address(&caller);

        let payment = self.call_value().single_esdt();
        self.require_is_base_asset_token(&payment.token_identifier);

        let base_asset_token_id = self.base_asset_token_id().get();
        let current_epoch = self.blockchain().get_block_epoch();

        let mut total_tokens_in_pairs = BigUint::zero();
        let mut total_unlockable_tokens = BigUint::zero();
        let mut output_payments = ManagedVec::new();
        let mut energy = self.get_updated_energy_entry_for_user(&original_caller, current_epoch);
        for pair in amount_unlock_epoch_pairs {
            let (token_amount, unlock_epoch) = pair.into_tuple();
            total_tokens_in_pairs += &token_amount;

            if unlock_epoch > current_epoch {
                energy.add_after_token_lock(&token_amount, unlock_epoch, current_epoch);

                let original_tokens = EgldOrEsdtTokenPayment::new(
                    EgldOrEsdtTokenIdentifier::esdt(base_asset_token_id.clone()),
                    0,
                    token_amount,
                );
                let locked_tokens = self.lock_tokens(original_tokens, unlock_epoch);
                output_payments.push(self.to_esdt_payment(locked_tokens));
            } else {
                total_unlockable_tokens += &token_amount;
            }
        }

        require!(
            payment.amount == total_tokens_in_pairs,
            "Total amount mismatch"
        );

        if total_unlockable_tokens > 0 {
            let unlockable_tokens_payment =
                EsdtTokenPayment::new(base_asset_token_id, 0, total_unlockable_tokens);
            output_payments.push(unlockable_tokens_payment);
        }

        if !output_payments.is_empty() {
            self.send().direct_multi(&original_caller, &output_payments);
        }

        self.user_energy(&original_caller).set(&energy);

        output_payments
    }

    fn require_old_sc_address(&self, address: &ManagedAddress) {
        let mapper = self.old_locked_asset_factory_address();
        require!(!mapper.is_empty(), "old SC address not set");

        let sc_address = mapper.get();
        require!(address == &sc_address, "Invalid SC address");
    }

    fn require_is_new_token(&self, token_nonce: Nonce) {
        let new_token_start_nonce = self.new_token_start_nonce().get();
        require!(
            token_nonce >= new_token_start_nonce,
            "Old tokens not allowed"
        );
    }

    #[storage_mapper("oldLockedAssetFactoryAddress")]
    fn old_locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("newTokenStartNonce")]
    fn new_token_start_nonce(&self) -> SingleValueMapper<Nonce>;
}
