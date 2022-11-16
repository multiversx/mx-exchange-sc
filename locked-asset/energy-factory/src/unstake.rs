elrond_wasm::imports!();

use crate::energy::Energy;

mod token_unstake_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait TokenUnstakeProxy {
        #[payable("*")]
        #[endpoint(depositUserTokens)]
        fn deposit_user_tokens(&self, user: ManagedAddress);

        #[payable("*")]
        #[endpoint(depositFees)]
        fn deposit_fees(&self);
    }
}

#[elrond_wasm::module]
pub trait UnstakeModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + simple_lock::token_attributes::TokenAttributesModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + crate::token_merging::TokenMergingModule
    + elrond_wasm_modules::pause::PauseModule
    + crate::penalty::LocalPenaltyModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
    + crate::token_whitelist::TokenWhitelistModule
{
    #[only_owner]
    #[endpoint(setTokenUnstakeAddress)]
    fn set_token_unstake_address(&self, sc_address: ManagedAddress) {
        self.require_sc_address(&sc_address);
        self.token_unstake_sc_address().set(&sc_address);
    }

    #[endpoint(createMergedLockedTokenForFees)]
    fn create_merged_locked_token_for_fees(
        &self,
        amount: BigUint,
        unlock_epoch: u64,
    ) -> EsdtTokenPayment {
        self.require_caller_unstake_sc();

        let caller = self.blockchain().get_caller();
        let base_asset_token_id = self.base_asset_token_id().get();
        let virtual_payment = EgldOrEsdtTokenPayment::new(
            EgldOrEsdtTokenIdentifier::esdt(base_asset_token_id),
            0,
            amount,
        );
        let output_payment = self.lock_and_send(&caller, virtual_payment, unlock_epoch);

        self.to_esdt_payment(output_payment)
    }

    #[payable("*")]
    #[endpoint(revertUnstake)]
    fn revert_unstake(&self, user: ManagedAddress, new_energy: Energy<Self::Api>) {
        self.require_caller_unstake_sc();

        self.set_energy_entry(&user, new_energy);
    }

    fn unstake_tokens(
        &self,
        caller: ManagedAddress,
        locked_tokens: EsdtTokenPayment,
        unlocked_tokens: EsdtTokenPayment,
    ) {
        let locking_sc_address = self.token_unstake_sc_address().get();
        let mut payments = ManagedVec::new();
        payments.push(locked_tokens);
        payments.push(unlocked_tokens);

        let _: IgnoreValue = self
            .token_unstake_sc_proxy_obj(locking_sc_address)
            .deposit_user_tokens(caller)
            .with_multi_token_transfer(payments)
            .execute_on_dest_context();
    }

    fn send_fees_to_unstake_sc(&self, fees: EsdtTokenPayment) {
        let locking_sc_address = self.token_unstake_sc_address().get();
        let _: IgnoreValue = self
            .token_unstake_sc_proxy_obj(locking_sc_address)
            .deposit_fees()
            .add_esdt_token_transfer(fees.token_identifier, fees.token_nonce, fees.amount)
            .execute_on_dest_context();
    }

    fn require_caller_unstake_sc(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.token_unstake_sc_address().get();
        require!(
            caller == sc_address,
            "Only the unstake SC may call this endpoint"
        );
    }

    #[proxy]
    fn token_unstake_sc_proxy_obj(
        &self,
        sc_address: ManagedAddress,
    ) -> token_unstake_proxy::Proxy<Self::Api>;

    #[view(getTokenUnstakeScAddress)]
    #[storage_mapper("tokenUnstakeScAddress")]
    fn token_unstake_sc_address(&self) -> SingleValueMapper<ManagedAddress>;
}
