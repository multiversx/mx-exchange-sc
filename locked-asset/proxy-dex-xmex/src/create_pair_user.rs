use router::{ProxyTrait as _, DEFAULT_SPECIAL_FEE_PERCENT, DEFAULT_TOTAL_FEE_PERCENT};

multiversx_sc::imports!();

const GAS_FOR_END_TX: u64 = 10_000;
const ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD

pub type GasLimit = u64;

#[multiversx_sc::module]
pub trait CreatePairUserModule:
    crate::other_sc_whitelist::OtherScWhitelistModule + energy_query::EnergyQueryModule
{
    #[only_owner]
    #[endpoint(clearTokenInfo)]
    fn clear_token_info(&self, token_id: TokenIdentifier) {
        self.requested_price(&token_id).clear();
        self.pair_for_token(&token_id).clear();
    }

    #[payable("*")]
    #[endpoint(depositProjectToken)]
    fn deposit_project_token(&self, requested_mex_price: BigUint) {
        let (token_id, _) = self.call_value().single_fungible_esdt();
        self.requested_price(&token_id).update(|price| {
            require!(*price == 0, "Price already set");

            *price = requested_mex_price;
        });
    }

    #[payable("EGLD")]
    #[endpoint(createXmexTokenPair)]
    fn create_xmex_token_pair(
        &self,
        token_id: TokenIdentifier,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    ) {
        require!(
            !self.requested_price(&token_id).is_empty(),
            "Tokens not deposited"
        );
        require!(
            self.pair_for_token(&token_id).is_empty(),
            "Pair already created"
        );

        let payment_amount = self.call_value().egld_value().clone_value();
        require!(payment_amount == ISSUE_COST, "Invalid payment amount");

        let caller = self.blockchain().get_caller();
        let pair_addr = self.create_pair(token_id, caller);
        let _ = self.intermediated_pairs().insert(pair_addr.clone());

        self.issue_pair_lp_token(pair_addr, lp_token_display_name, lp_token_ticker);
    }

    #[endpoint(setPairLocalRoles)]
    fn set_pair_local_roles(&self, pair_address: ManagedAddress) {
        let router_address = self.router_address().get();
        let gas_for_call = self.get_async_call_gas();
        let _: IgnoreValue = self
            .router_proxy(router_address)
            .set_local_roles(pair_address)
            .with_gas_limit(gas_for_call)
            .execute_on_dest_context();
    }

    fn create_pair(&self, token_id: TokenIdentifier, caller: ManagedAddress) -> ManagedAddress {
        let mex_token_id = self.get_base_token_id();
        let own_sc_address = self.blockchain().get_sc_address();
        let router_address = self.router_address().get();
        let opt_fee_percents = OptionalValue::Some(MultiValue2::from((
            DEFAULT_TOTAL_FEE_PERCENT,
            DEFAULT_SPECIAL_FEE_PERCENT,
        )));

        let mut admins = MultiValueEncoded::new();
        admins.push(caller);

        // opt_fee_percents is ignored for non-owner callers
        self.router_proxy(router_address)
            .create_pair_endpoint(
                token_id,
                mex_token_id,
                own_sc_address,
                opt_fee_percents,
                admins,
            )
            .execute_on_dest_context()
    }

    fn issue_pair_lp_token(
        &self,
        pair_address: ManagedAddress,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    ) {
        let router_address = self.router_address().get();
        let gas_for_call = self.get_async_call_gas();
        let _: IgnoreValue = self
            .router_proxy(router_address)
            .issue_lp_token(pair_address, lp_token_display_name, lp_token_ticker)
            .with_gas_limit(gas_for_call)
            .execute_on_dest_context();
    }

    fn get_async_call_gas(&self) -> GasLimit {
        let remaining_gas = self.blockchain().get_gas_left();
        require!(remaining_gas > GAS_FOR_END_TX, "Not enough gas");

        remaining_gas - GAS_FOR_END_TX
    }

    #[proxy]
    fn router_proxy(&self, sc_address: ManagedAddress) -> router::Proxy<Self::Api>;

    #[storage_mapper("routerAddr")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getRequestedPrice)]
    #[storage_mapper("reqPrice")]
    fn requested_price(&self, token_id: &TokenIdentifier) -> SingleValueMapper<BigUint>;

    #[view(getPairForToken)]
    #[storage_mapper("pairForToken")]
    fn pair_for_token(&self, token_id: &TokenIdentifier) -> SingleValueMapper<ManagedAddress>;
}
