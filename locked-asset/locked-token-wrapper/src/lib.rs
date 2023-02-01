#![no_std]

multiversx_sc::imports!();

pub mod wrapped_token;

#[multiversx_sc::contract]
pub trait LockedTokenWrapper:
    wrapped_token::WrappedTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + lkmex_transfer::energy_transfer::EnergyTransferModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
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
        let caller = self.blockchain().get_caller();
        require!(
            !self.blockchain().is_smart_contract(&caller),
            "SCs cannot unwrap locked tokens"
        );

        let payment = self.call_value().single_esdt();
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
