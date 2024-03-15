use router::{ProxyTrait as _, DEFAULT_SPECIAL_FEE_PERCENT, DEFAULT_TOTAL_FEE_PERCENT};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const GAS_FOR_END_TX: u64 = 10_000;
pub const ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.05 EGLD

pub static TOKENS_NOT_DEPOSITED_ERR_MSG: &[u8] = b"Tokens not deposited";

pub type GasLimit = u64;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct TokenInfo<M: ManagedTypeApi> {
    pub depositor: ManagedAddress<M>,
    pub deposited_tokens: BigUint<M>,
    pub requested_price: BigUint<M>,
    pub opt_pair: Option<ManagedAddress<M>>,
}

#[multiversx_sc::module]
pub trait CreatePairUserModule:
    proxy_dex::other_sc_whitelist::OtherScWhitelistModule
    + energy_query::EnergyQueryModule
    + token_send::TokenSendModule
{
    /// note: If you ever clear a token that already created the pair, you need to manually remove it from router
    #[only_owner]
    #[endpoint(clearTokenInfo)]
    fn clear_token_info(&self, token_id: TokenIdentifier) {
        let token_info = self.token_info(&token_id).take();
        self.send_tokens_non_zero(
            &token_info.depositor,
            &token_id,
            0,
            &token_info.deposited_tokens,
        );

        if token_info.opt_pair.is_some() {
            self.send()
                .direct_egld(&token_info.depositor, &BigUint::from(ISSUE_COST));
        }
    }

    #[payable("*")]
    #[endpoint(depositProjectToken)]
    fn deposit_project_token(&self, requested_mex_price: BigUint) {
        let (token_id, amount) = self.call_value().single_fungible_esdt();
        let info_mapper = self.token_info(&token_id);
        require!(info_mapper.is_empty(), "Price already set");

        let caller = self.blockchain().get_caller();
        let token_info = TokenInfo {
            depositor: caller,
            deposited_tokens: amount,
            requested_price: requested_mex_price,
            opt_pair: None,
        };
        info_mapper.set(token_info);
    }

    #[payable("EGLD")]
    #[endpoint(createXmexTokenPair)]
    fn create_xmex_token_pair(
        &self,
        token_id: TokenIdentifier,
        lp_token_display_name: ManagedBuffer,
        lp_token_ticker: ManagedBuffer,
    ) {
        let info_mapper = self.token_info(&token_id);
        require!(!info_mapper.is_empty(), TOKENS_NOT_DEPOSITED_ERR_MSG);

        let mut token_info = info_mapper.get();
        require!(token_info.opt_pair.is_none(), "Pair already created");

        let payment_amount = self.call_value().egld_value().clone_value();
        require!(payment_amount == ISSUE_COST, "Invalid payment amount");

        let caller = self.blockchain().get_caller();
        let pair_addr = self.create_pair(token_id, caller);
        let added = self.intermediated_pairs().insert(pair_addr.clone());
        require!(added, "Pair already exists");

        token_info.opt_pair = Some(pair_addr.clone());

        info_mapper.set(token_info);

        // Comment this line if you want tests to work
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

    #[view(getPairAddress)]
    fn get_pair_address(&self, token_id: TokenIdentifier) -> OptionalValue<ManagedAddress> {
        let info_mapper = self.token_info(&token_id);
        if info_mapper.is_empty() {
            return OptionalValue::None;
        }

        let token_info = info_mapper.get();
        token_info.opt_pair.into()
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

    #[view(getTokenInfo)]
    #[storage_mapper("tokenInfo")]
    fn token_info(&self, token_id: &TokenIdentifier) -> SingleValueMapper<TokenInfo<Self::Api>>;
}
