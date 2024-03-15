multiversx_sc::imports!();
multiversx_sc::derive_imports!();

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
    fn create_xmex_token_pair(&self, pair_address: ManagedAddress, token_id: TokenIdentifier) {
        let info_mapper = self.token_info(&token_id);
        require!(!info_mapper.is_empty(), TOKENS_NOT_DEPOSITED_ERR_MSG);

        let mut token_info = info_mapper.get();
        require!(token_info.opt_pair.is_none(), "Pair already created");

        let payment_amount = self.call_value().egld_value().clone_value();
        require!(payment_amount == ISSUE_COST, "Invalid payment amount");

        let caller = self.blockchain().get_caller();
        require!(caller == token_info.depositor, "Invalid caller");

        let added = self.intermediated_pairs().insert(pair_address.clone());
        require!(added, "Pair already exists");

        token_info.opt_pair = Some(pair_address.clone());

        info_mapper.set(token_info);
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

    #[view(getTokenInfo)]
    #[storage_mapper("tokenInfo")]
    fn token_info(&self, token_id: &TokenIdentifier) -> SingleValueMapper<TokenInfo<Self::Api>>;
}
