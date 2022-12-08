#![no_std]

elrond_wasm::imports!();

pub mod wrapped_token;

#[elrond_wasm::contract]
pub trait LockedTokenWrapper:
    wrapped_token::WrappedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + lkmex_transfer::energy_transfer::EnergyTransferModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    #[init]
    fn init(
        &self,
        old_locked_token_id: TokenIdentifier,
        new_locked_token_id: TokenIdentifier,
        energy_factory_address: ManagedAddress,
    ) {
        self.require_valid_token_id(&old_locked_token_id);
        self.require_valid_token_id(&new_locked_token_id);
        self.require_sc_address(&energy_factory_address);

        if self.old_locked_token().is_empty() {
            self.old_locked_token().set_token_id(old_locked_token_id);
        }
        if self.locked_token().is_empty() {
            self.locked_token().set_token_id(new_locked_token_id);
        }
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[payable("*")]
    #[endpoint(wrapLockedToken)]
    fn wrap_locked_token_endpoint(&self) -> EsdtTokenPayment {
        let payment = self.call_value().single_esdt();
        let caller = self.blockchain().get_caller();

        if payment.token_identifier == self.locked_token().get_token_id() {
            self.deduct_energy_from_sender(
                caller.clone(),
                &ManagedVec::from_single_item(payment.clone()),
            );
            self.wrap_locked_token_and_send(&caller, payment)
        } else if payment.token_identifier == self.old_locked_token().get_token_id() {
            self.deduct_old_token_energy_from_sender(
                caller.clone(),
                &ManagedVec::from_single_item(payment.clone()),
            );

            self.wrap_locked_token_and_send(&caller, payment)
        } else {
            sc_panic!("Bad payment token");
        }
    }

    #[payable("*")]
    #[endpoint(unwrapLockedToken)]
    fn unwrap_locked_token_endpoint(&self) -> EsdtTokenPayment {
        let payment = self.call_value().single_esdt();
        let caller = self.blockchain().get_caller();
        let original_locked_tokens = self.unwrap_locked_token(payment);

        if original_locked_tokens.token_identifier == self.locked_token().get_token_id() {
            self.add_energy_to_destination(
                caller.clone(),
                &ManagedVec::from_single_item(original_locked_tokens.clone()),
            );
        } else if original_locked_tokens.token_identifier == self.old_locked_token().get_token_id()
        {
            self.add_old_token_energy_to_destination(
                caller.clone(),
                &ManagedVec::from_single_item(original_locked_tokens.clone()),
            );
        }

        self.send().direct_esdt(
            &caller,
            &original_locked_tokens.token_identifier,
            original_locked_tokens.token_nonce,
            &original_locked_tokens.amount,
        );

        original_locked_tokens
    }
}
