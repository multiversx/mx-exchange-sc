elrond_wasm::imports!();

use common_structs::Epoch;

mod simple_lock_energy_proxy {
    elrond_wasm::imports!();

    use common_structs::Epoch;

    #[elrond_wasm::proxy]
    pub trait SimpleLockEnergyProxy {
        #[payable("*")]
        #[endpoint(acceptMigratedTokens)]
        fn accept_migrated_tokens(
            &self,
            original_caller: ManagedAddress,
            amount_unlock_epoch_pairs: MultiValueEncoded<MultiValue2<BigUint, Epoch>>,
        ) -> ManagedVec<EsdtTokenPayment<Self::Api>>;
    }
}

#[elrond_wasm::module]
pub trait LockedTokenMigrationModule:
    crate::locked_asset::LockedAssetModule
    + token_send::TokenSendModule
    + crate::attr_ex_helper::AttrExHelper
    + elrond_wasm_modules::pause::PauseModule
{
    /// This endpoint allows migration to the new SC to start, which in turn:
    /// - sets the address of the new factory, which should be a SimpleLockEnergy SC
    /// - pauses locked asset factory
    #[only_owner]
    #[endpoint(startMigration)]
    fn start_migration(&self, new_sc_address: ManagedAddress) {
        require!(
            self.new_contract_address().is_empty(),
            "Migration already started"
        );
        require!(
            self.blockchain().is_smart_contract(&new_sc_address),
            "Invalid SC address"
        );

        self.new_contract_address().set(&new_sc_address);
        self.set_paused(true);
    }

    /// Facilitates migrating of old locked tokens to the new contract.
    ///
    /// Expected input payments: Any number of locked tokens
    ///
    /// Output payments: New version of the locked tokens.
    /// The new tokens may be used in the new contract, which can be queried via getNewContractAddress
    #[payable("*")]
    #[endpoint(migrateToNewFactory)]
    fn migrate_to_new_factory(&self) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        self.require_paused();

        let payments = self.call_value().all_esdt_transfers();
        require!(!payments.is_empty(), "No payments");

        let locked_token_id = self.locked_asset_token().get_token_id();
        let mut total_locked_tokens = BigUint::zero();
        let mut args = MultiValueEncoded::new();
        for payment in &payments {
            require!(payment.token_identifier == locked_token_id, "Invalid token");

            let attributes = self.get_attributes_ex(&payment.token_identifier, payment.token_nonce);
            let unlock_amounts = self.get_unlock_amounts_per_milestone(
                &attributes.unlock_schedule.unlock_milestones,
                &payment.amount,
            );
            for pair in unlock_amounts {
                if pair.amount > 0 {
                    args.push((pair.amount, pair.epoch).into());
                }
            }

            self.send().esdt_local_burn(
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
            );

            total_locked_tokens += payment.amount;
        }

        self.migrate_tokens(total_locked_tokens, args)
    }

    fn migrate_tokens(
        &self,
        total_base_asset_amount: BigUint,
        arg_pairs: MultiValueEncoded<MultiValue2<BigUint, Epoch>>,
    ) -> ManagedVec<EsdtTokenPayment<Self::Api>> {
        let original_caller = self.blockchain().get_caller();
        let base_asset_token_id = self.asset_token_id().get();
        let sc_address = self.new_contract_address().get();

        self.simple_lock_energy_proxy_builder(sc_address)
            .accept_migrated_tokens(original_caller, arg_pairs)
            .add_esdt_token_transfer(base_asset_token_id, 0, total_base_asset_amount)
            .execute_on_dest_context()
    }

    #[proxy]
    fn simple_lock_energy_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> simple_lock_energy_proxy::Proxy<Self::Api>;

    #[view(getNewContractAddress)]
    #[storage_mapper("newContractAddress")]
    fn new_contract_address(&self) -> SingleValueMapper<ManagedAddress>;
}
