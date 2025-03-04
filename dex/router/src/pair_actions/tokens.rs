use pair::config::ProxyTrait as _;

multiversx_sc::imports!();

pub const LP_TOKEN_DECIMALS: usize = 18;
pub const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

#[multiversx_sc::module]
pub trait TokensModule:
    crate::config::ConfigModule
    + pair::read_pair_storage::ReadPairStorageModule
    + crate::temp_owner::TempOwnerModule
    + crate::state::StateModule
{
    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        pair_address: ManagedAddress,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    ) {
        self.require_active();

        let issue_cost = self.call_value().egld_value().clone_value();
        let caller = self.blockchain().get_caller();
        if caller != self.owner().get() {
            self.require_pair_creation_enabled();
        }

        self.check_is_pair_sc(&pair_address);

        let result = self.get_pair_temporary_owner(&pair_address);
        match result {
            None => {}
            Some(temporary_owner) => {
                require!(caller == temporary_owner, "Temporary owner differs");
            }
        };

        let get_lp_result: TokenIdentifier = self
            .pair_contract_proxy_tokens(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context();
        require!(
            !get_lp_result.is_valid_esdt_identifier(),
            "LP Token already issued"
        );

        self.send()
            .esdt_system_sc_proxy()
            .issue_fungible(
                issue_cost,
                &lp_token_display_name,
                &lp_token_ticker,
                &BigUint::from(LP_TOKEN_INITIAL_SUPPLY),
                FungibleTokenProperties {
                    num_decimals: LP_TOKEN_DECIMALS,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .with_callback(
                self.callbacks()
                    .lp_token_issue_callback(&caller, &pair_address),
            )
            .async_call_and_exit()
    }

    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, pair_address: ManagedAddress) {
        self.require_active();
        self.check_is_pair_sc(&pair_address);

        let pair_token: TokenIdentifier = self
            .pair_contract_proxy_tokens(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context();
        require!(pair_token.is_valid_esdt_identifier(), "LP token not issued");

        let roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(&pair_address, &pair_token, roles.iter().cloned())
            .async_call_and_exit()
    }

    #[callback]
    fn lp_token_issue_callback(
        &self,
        caller: &ManagedAddress,
        address: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<()>,
    ) {
        let (token_id, returned_tokens) = self.call_value().egld_or_single_fungible_esdt();
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.pair_temporary_owner().remove(address);

                let _: IgnoreValue = self
                    .pair_contract_proxy_tokens(address.clone())
                    .set_lp_token_identifier(token_id.unwrap_esdt())
                    .execute_on_dest_context();
            }
            ManagedAsyncCallResult::Err(_) => {
                if token_id.is_egld() && returned_tokens > 0u64 {
                    self.send().direct_egld(caller, &returned_tokens);
                }
            }
        }
    }

    #[proxy]
    fn pair_contract_proxy_tokens(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
