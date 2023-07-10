#![no_std]

multiversx_sc::imports!();

use itertools::Itertools;

type AddLiquidityResultType<BigUint> =
    MultiValue3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub const MINIMUM_LIQUIDITY: u64 = 1_000;
pub const DEFAULT_FIRST_TOKEN_ID: &[u8] = b"FIRST-abcdef";
pub const DEFAULT_SECOND_TOKEN_ID: &[u8] = b"SECOND-abcdef";
pub const DEFAULT_LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";
pub const DEFAULT_STATE: bool = true;
pub const DEFAULT_SKIP_MINTING_LP_TOKENS: bool = true;

#[multiversx_sc::derive::contract]
pub trait PairMock {
    #[init]
    fn init(
        &self,
        first_token_id: OptionalValue<TokenIdentifier>,
        second_token_id: OptionalValue<TokenIdentifier>,
        lp_token_id: OptionalValue<TokenIdentifier>,
        initial_liquidity_adder: OptionalValue<ManagedAddress>,
        state: OptionalValue<bool>,
        skip_minting_lp_tokens: OptionalValue<bool>,
    ) {
        self.first_token_id().set(
            first_token_id
                .into_option()
                .as_ref()
                .unwrap_or(&TokenIdentifier::from_esdt_bytes(DEFAULT_FIRST_TOKEN_ID)),
        );
        self.second_token_id().set(
            second_token_id
                .into_option()
                .as_ref()
                .unwrap_or(&TokenIdentifier::from_esdt_bytes(DEFAULT_SECOND_TOKEN_ID)),
        );
        self.lp_token_id().set(
            lp_token_id
                .into_option()
                .as_ref()
                .unwrap_or(&TokenIdentifier::from_esdt_bytes(DEFAULT_LP_TOKEN_ID)),
        );
        self.initial_liquidity_adder().set(
            initial_liquidity_adder
                .into_option()
                .as_ref()
                .unwrap_or(&self.blockchain().get_caller()),
        );
        self.state()
            .set(state.into_option().as_ref().unwrap_or(&DEFAULT_STATE));
        self.skip_minting_lp_tokens().set(
            skip_minting_lp_tokens
                .into_option()
                .as_ref()
                .unwrap_or(&DEFAULT_SKIP_MINTING_LP_TOKENS),
        );
    }

    #[payable("*")]
    #[endpoint(addInitialLiquidity)]
    fn add_initial_liquidity(&self) -> AddLiquidityResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();
        require!(self.state().get(), "Inactive");

        let lp_token_id = self.lp_token_id().get();
        require!(
            lp_token_id.is_valid_esdt_identifier(),
            "LP token not issued"
        );

        let (first_payment, second_payment) = payments
            .into_iter()
            .collect_tuple()
            .ok_or("bad payments len")
            .unwrap();

        let expected_first_token_id = self.first_token_id().get();
        let expected_second_token_id = self.second_token_id().get();

        require!(
            first_payment.token_identifier == expected_first_token_id
                && first_payment.token_identifier != lp_token_id,
            "bad first payment"
        );
        require!(
            second_payment.token_identifier == expected_second_token_id
                && second_payment.token_identifier != lp_token_id,
            "bad second payment"
        );
        require!(first_payment.token_nonce == 0, "non zero first token nonce");
        require!(
            second_payment.token_nonce == 0,
            "non zero second token nonce"
        );

        let liquidity = core::cmp::min(first_payment.amount, second_payment.amount);
        require!(liquidity > MINIMUM_LIQUIDITY, "Minimum liquidity");

        if !self.skip_minting_lp_tokens().get() {
            self.send().esdt_local_mint(&lp_token_id, 0, &liquidity);
        }

        self.lp_token_supply().set(&liquidity);

        let caller = self.blockchain().get_caller();

        let lp_token_amount = liquidity - MINIMUM_LIQUIDITY;
        self.send()
            .direct_esdt(&caller, &lp_token_id, 0, &lp_token_amount);

        (
            EsdtTokenPayment::new(lp_token_id, 0, lp_token_amount),
            EsdtTokenPayment::new(expected_first_token_id, 0, BigUint::zero()),
            EsdtTokenPayment::new(expected_second_token_id, 0, BigUint::zero()),
        )
            .into()
    }

    #[endpoint(getTokensForGivenPositionWithSafePrice)]
    fn get_tokens_for_given_position_with_safe_price(
        &self,
        liquidity: BigUint,
    ) -> MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> {
        (
            EsdtTokenPayment::new(self.first_token_id().get(), 0, liquidity.clone() / 2u64),
            EsdtTokenPayment::new(self.second_token_id().get(), 0, liquidity / 2u64),
        )
            .into()
    }

    #[storage_mapper("first_token_id")]
    fn first_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("second_token_id")]
    fn second_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("lp_token_id")]
    fn lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("lp_token_supply")]
    fn lp_token_supply(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("initial_liquidity_adder")]
    fn initial_liquidity_adder(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<bool>;

    #[storage_mapper("skip_minting_lp_tokens")]
    fn skip_minting_lp_tokens(&self) -> SingleValueMapper<bool>;
}
