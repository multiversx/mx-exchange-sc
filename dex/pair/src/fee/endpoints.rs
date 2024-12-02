use common_structs::{Percent, TokenPair};

use crate::{
    config::MAX_PERCENTAGE, ERROR_ALREADY_FEE_DEST, ERROR_ALREADY_WHITELISTED,
    ERROR_BAD_TOKEN_FEE_DEST, ERROR_NOT_FEE_DEST, ERROR_NOT_WHITELISTED,
    ERROR_PAIR_ALREADY_TRUSTED, ERROR_PAIR_NOT_TRUSTED, ERROR_SAME_TOKENS,
};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EndpointsModule:
    crate::config::ConfigModule
    + crate::liquidity_pool::LiquidityPoolModule
    + crate::amm::AmmModule
    + token_send::TokenSendModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + super::storage::StorageModule
{
    #[endpoint(whitelist)]
    fn whitelist_endpoint(&self, address: ManagedAddress) {
        self.require_caller_has_owner_permissions();

        let is_new = self.whitelist().insert(address);
        require!(is_new, ERROR_ALREADY_WHITELISTED);
    }

    #[endpoint(removeWhitelist)]
    fn remove_whitelist(&self, address: ManagedAddress) {
        self.require_caller_has_owner_permissions();

        let is_removed = self.whitelist().remove(&address);
        require!(is_removed, ERROR_NOT_WHITELISTED);
    }

    #[endpoint(addTrustedSwapPair)]
    fn add_trusted_swap_pair(
        &self,
        pair_address: ManagedAddress,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) {
        self.require_caller_has_owner_permissions();
        require!(first_token != second_token, ERROR_SAME_TOKENS);

        let token_pair = TokenPair {
            first_token,
            second_token,
        };
        let is_new = self
            .trusted_swap_pair()
            .insert(token_pair, pair_address)
            .is_none();
        require!(is_new, ERROR_PAIR_ALREADY_TRUSTED);
    }

    #[endpoint(removeTrustedSwapPair)]
    fn remove_trusted_swap_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
    ) {
        self.require_caller_has_owner_permissions();

        let token_pair = TokenPair {
            first_token: first_token.clone(),
            second_token: second_token.clone(),
        };

        let mut is_removed = self.trusted_swap_pair().remove(&token_pair).is_some();
        if !is_removed {
            let token_pair_reversed = TokenPair {
                first_token: second_token,
                second_token: first_token,
            };
            is_removed = self
                .trusted_swap_pair()
                .remove(&token_pair_reversed)
                .is_some();
            require!(is_removed, ERROR_PAIR_NOT_TRUSTED);
        }
    }

    /// `fees_collector_cut_percentage` of the special fees are sent to the fees_collector_address SC
    ///
    /// For example, if special fees is 5%, and fees_collector_cut_percentage is 10%,
    /// then of the 5%, 10% are reserved, and only the rest are split between other pair contracts.
    #[endpoint(setupFeesCollector)]
    fn setup_fees_collector(
        &self,
        fees_collector_address: ManagedAddress,
        fees_collector_cut_percentage: Percent,
    ) {
        self.require_caller_has_owner_permissions();
        require!(
            self.blockchain().is_smart_contract(&fees_collector_address),
            "Invalid fees collector address"
        );
        require!(
            fees_collector_cut_percentage > 0 && fees_collector_cut_percentage <= MAX_PERCENTAGE,
            "Invalid fees percentage"
        );

        self.fees_collector_address().set(&fees_collector_address);
        self.fees_collector_cut_percentage()
            .set(fees_collector_cut_percentage);
    }

    #[endpoint(setFeeOn)]
    fn set_fee_on(&self, fee_to_address: ManagedAddress, fee_token: TokenIdentifier) {
        self.require_caller_has_owner_permissions();

        let is_dest = self
            .destination_map()
            .keys()
            .any(|dest_address| dest_address == fee_to_address);
        require!(!is_dest, ERROR_ALREADY_FEE_DEST);

        let _ = self.destination_map().insert(fee_to_address, fee_token);
    }

    #[endpoint(setFeeOn)]
    fn set_fee_off(&self, fee_to_address: ManagedAddress, fee_token: TokenIdentifier) {
        self.require_caller_has_owner_permissions();

        let is_dest = self
            .destination_map()
            .keys()
            .any(|dest_address| dest_address == fee_to_address);
        require!(is_dest, ERROR_NOT_FEE_DEST);

        let dest_fee_token = self.destination_map().get(&fee_to_address).unwrap();
        require!(fee_token == dest_fee_token, ERROR_BAD_TOKEN_FEE_DEST);

        let _ = self.destination_map().remove(&fee_to_address);
    }
}
