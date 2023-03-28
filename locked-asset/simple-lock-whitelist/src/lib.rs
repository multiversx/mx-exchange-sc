#![no_std]

use simple_lock::error_messages::INVALID_PAYMENTS_ERR_MSG;

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait SimpleLockWhitelist:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::proxy_lp::ProxyLpModule
    + simple_lock::proxy_farm::ProxyFarmModule
    + simple_lock::lp_interactions::LpInteractionsModule
    + simple_lock::farm_interactions::FarmInteractionsModule
    + simple_lock::token_attributes::TokenAttributesModule
    + utils::UtilsModule
{
    /// Args: Token IDs that are accepted for the `lock` endpoint.
    /// Any other token is rejected.
    #[init]
    fn init(&self, token_whitelist: MultiValueEncoded<TokenIdentifier>) {
        let mut whitelist = self.token_whitelist();
        for token_id in token_whitelist {
            self.require_valid_token_id(&token_id);

            let _ = whitelist.insert(token_id);
        }
    }

    /// Sets the transfer role for the given address. Defaults to own address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedToken)]
    fn set_transfer_role(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.locked_token()
            .set_local_roles_for_address(&address, &[EsdtLocalRole::Transfer], None);
    }

    #[only_owner]
    #[endpoint(setTransferRoleProxyLpToken)]
    fn set_transfer_role_proxy_lp(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.lp_proxy_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            None,
        );
    }

    #[only_owner]
    #[endpoint(setTransferRoleProxyFarmToken)]
    fn set_transfer_role_proxy_farm(&self, opt_address: OptionalValue<ManagedAddress>) {
        let address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };

        self.farm_proxy_token().set_local_roles_for_address(
            &address,
            &[EsdtLocalRole::Transfer],
            None,
        );
    }

    #[only_owner]
    #[endpoint(setLockedToken)]
    fn set_locked_token(&self, token_id: TokenIdentifier) {
        require!(token_id.is_valid_esdt_identifier(), "Token id is not valid");
        self.locked_token().set_token_id(token_id);
    }

    /// Locks a whitelisted token until `unlock_epoch` and receive meta ESDT LOCKED tokens.
    /// on a 1:1 ratio. If unlock epoch has already passed, the original tokens are sent instead.
    ///
    /// Expected payment: A whitelisted token
    ///
    /// Arguments:
    /// - unlock epoch - the epoch from which the LOCKED token holder may call the unlock endpoint
    /// - opt_destination - OPTIONAL: destination address for the LOCKED tokens. Default is caller.
    ///
    /// Output payments: LOCKED tokens (or original payment if current_epoch >= unlock_epoch)
    #[payable("*")]
    #[endpoint(lockTokens)]
    fn lock_tokens_endpoint(
        &self,
        unlock_epoch: u64,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment = self.call_value().single_esdt();
        self.require_token_in_whitelist(&payment.token_identifier);

        let dest_address = self.dest_from_optional(opt_destination);
        self.lock_and_send(
            &dest_address,
            EgldOrEsdtTokenPayment::from(payment),
            unlock_epoch,
        )
    }

    /// Unlock tokens, previously locked with the `lockTokens` endpoint
    ///
    /// Expected payment: LOCKED tokens
    ///
    /// Arguments:
    /// - opt_destination - OPTIONAL: destination address for the unlocked tokens
    ///
    /// Output payments: the originally locked tokens
    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens_endpoint(
        &self,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment = self.call_value().single_esdt();
        let dest_address = self.dest_from_optional(opt_destination);
        self.unlock_and_send(&dest_address, payment)
    }

    fn require_token_in_whitelist(&self, token_id: &TokenIdentifier) {
        require!(
            self.token_whitelist().contains(token_id),
            INVALID_PAYMENTS_ERR_MSG
        );
    }

    #[view(getTokenWhitelist)]
    #[storage_mapper("tokenWhitelist")]
    fn token_whitelist(&self) -> UnorderedSetMapper<TokenIdentifier>;
}
