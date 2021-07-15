elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::elgd_wrap_proxy;

use common_structs::{FftTokenAmountPair, TokenPair};

pub const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct PairContractImmutableInfo {
    token_pair: TokenPair,
    lp_token_id: TokenIdentifier,
    total_fee_percent: u64,
    special_fee_percent: u64,
    fee_base_points: u64,
}

#[elrond_wasm_derive::module]
pub trait PairHelperModule: elgd_wrap_proxy::EgldWrapProxyModule {
    #[proxy]
    fn pair_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;

    #[payable("*")]
    #[endpoint(acceptPay)]
    fn accept_pay(&self) {}

    #[payable("*")]
    #[endpoint(addLiquiditySingleToken)]
    fn add_liquidity_single_token(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: Self::BigUint,
        pair_address: Address,
    ) -> SCResult<()> {
        require!(amount_in != 0, "Amount in is zero");
        require!(
            self.intermediated_pairs().contains_key(&pair_address),
            "Not an intermediated pair"
        );
        let pair_info = self.intermediated_pairs().get(&pair_address).unwrap();
        require!(
            token_in == pair_info.token_pair.first_token
                || token_in == pair_info.token_pair.second_token,
            "Bad input token"
        );
        let caller = self.blockchain().get_caller();

        let (first_token_reserve, _, _) = self
            .pair_proxy(pair_address.clone())
            .get_reserves_and_total_supply()
            .execute_on_dest_context()
            .into_tuple();

        let swap_amount = self.compute_swap_amount(
            &amount_in,
            &first_token_reserve,
            pair_info.fee_base_points - pair_info.total_fee_percent,
            pair_info.fee_base_points,
        );
        require!(swap_amount != 0, "Swap amount is zero");
        require!(swap_amount < amount_in, "Swap amount too big");

        let swapped_tokens = self.swap(&token_in, &swap_amount, &pair_info, &pair_address);
        require!(
            swapped_tokens.amount != 0,
            "Received zero amount after swap"
        );

        let (liquidity, amount_in_leftover) = self.add_liq(
            &token_in,
            &(&amount_in - &swap_amount),
            &swapped_tokens.amount,
            &pair_info,
            &pair_address,
        );
        require!(
            liquidity.amount != 0,
            "Received zero amount after add liquidity"
        );

        if amount_in_leftover != 0 {
            self.send()
                .direct(&caller, &token_in, &amount_in_leftover, &[]);
        }

        self.send()
            .direct(&caller, &liquidity.token_id, &liquidity.amount, &[]);

        Ok(())
    }

    #[payable("*")]
    #[endpoint(removeLiquiditySingleToken)]
    fn remove_liquidity_single_token(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: Self::BigUint,
        desired_token: TokenIdentifier,
        pair_address: Address,
    ) -> SCResult<()> {
        require!(amount_in != 0, "Amount in is zero");
        require!(
            self.intermediated_pairs().contains_key(&pair_address),
            "Not an intermediated pair"
        );
        let pair_info = self.intermediated_pairs().get(&pair_address).unwrap();
        require!(token_in == pair_info.lp_token_id, "Bad input token");
        require!(
            desired_token == pair_info.token_pair.first_token
                || desired_token == pair_info.token_pair.second_token,
            "Bad desired token"
        );
        let caller = self.blockchain().get_caller();

        let (first_token, second_token) = self.rem_liquidity(&token_in, &amount_in, &pair_address);

        let desired_token_amount = if desired_token == first_token.token_id {
            let swapped_token = self.swap(
                &second_token.token_id,
                &second_token.amount,
                &pair_info,
                &pair_address,
            );
            swapped_token.amount + first_token.amount
        } else {
            let swapped_token = self.swap(
                &first_token.token_id,
                &first_token.amount,
                &pair_info,
                &pair_address,
            );
            swapped_token.amount + second_token.amount
        };

        self.send()
            .direct(&caller, &desired_token, &desired_token_amount, &[]);

        Ok(())
    }

    #[payable("*")]
    #[endpoint(removeLiquidityAndUnwrapWrappedEgld)]
    fn remove_liquidity_and_unwrap_wrapped_egld(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment_amount] amount_in: Self::BigUint,
        pair_address: Address,
    ) -> SCResult<()> {
        require!(amount_in != 0, "Amount in is zero");
        require!(
            self.intermediated_pairs().contains_key(&pair_address),
            "Not an intermediated pair"
        );
        let pair_info = self.intermediated_pairs().get(&pair_address).unwrap();
        require!(token_in == pair_info.lp_token_id, "Bad input token");
        let wegld_token_id = self.wegld_token_id().get();
        require!(
            wegld_token_id == pair_info.token_pair.first_token
                || wegld_token_id == pair_info.token_pair.second_token,
            "Pair tokens do not contain wegld"
        );
        let caller = self.blockchain().get_caller();

        let (first_token, second_token) = self.rem_liquidity(&token_in, &amount_in, &pair_address);

        let (wegld_token, other_token) = if first_token.token_id == wegld_token_id {
            (first_token, second_token)
        } else {
            (second_token, first_token)
        };

        self.unwrap_egld(&wegld_token.amount);
        self.send()
            .direct(&caller, &TokenIdentifier::egld(), &wegld_token.amount, &[]);
        self.send()
            .direct(&caller, &other_token.token_id, &other_token.amount, &[]);

        Ok(())
    }

    fn rem_liquidity(
        &self,
        lp_token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        pair_address: &Address,
    ) -> (
        FftTokenAmountPair<Self::BigUint>,
        FftTokenAmountPair<Self::BigUint>,
    ) {
        self.pair_proxy(pair_address.clone())
            .remove_liquidity(
                lp_token_id.clone(),
                amount.clone(),
                1u64.into(),
                1u64.into(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
            .into_tuple()
    }

    /*
        (r + x) / (s - (s * c * x / (x * c + r * t))) = (a - x) / (s * c * x / (x * c  + r * t)), x > 0, a > x, r > 0, s > 0, c > 0, t > 0
        a>0, c>0, r>0, s>0, t>0, x = (c sqrt((r (4 a c t + c^2 r + 2 c r t + r t^2))/c^2) + c (-r) - r t)/(2 c)
    */
    fn compute_swap_amount(
        &self,
        _a: &Self::BigUint,
        _r: &Self::BigUint,
        _c: u64,
        _t: u64,
    ) -> Self::BigUint {
        //TODO: Need sqrt
        0u64.into()
    }

    fn swap(
        &self,
        token_in: &TokenIdentifier,
        amount_in: &Self::BigUint,
        pair_info: &PairContractImmutableInfo,
        pair_address: &Address,
    ) -> FftTokenAmountPair<Self::BigUint> {
        let desired_token_id = if token_in != &pair_info.token_pair.first_token {
            &pair_info.token_pair.first_token
        } else {
            &pair_info.token_pair.second_token
        };

        self.pair_proxy(pair_address.clone())
            .swap_tokens_fixed_input(
                token_in.clone(),
                amount_in.clone(),
                desired_token_id.clone(),
                1u64.into(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
    }

    fn add_liq(
        &self,
        payment_token_id: &TokenIdentifier,
        payment_amount_left: &Self::BigUint,
        swapped_amount: &Self::BigUint,
        pair_info: &PairContractImmutableInfo,
        pair_address: &Address,
    ) -> (FftTokenAmountPair<Self::BigUint>, Self::BigUint) {
        let (
            first_token_amount,
            second_token_amount,
            first_token_amount_min,
            second_token_amount_min,
        ) = if payment_token_id == &pair_info.token_pair.first_token {
            (
                payment_amount_left.clone(),
                swapped_amount.clone(),
                payment_amount_left / &2u64.into() + 1u64.into(),
                swapped_amount.clone(),
            )
        } else {
            (
                swapped_amount.clone(),
                payment_amount_left.clone(),
                swapped_amount.clone(),
                payment_amount_left / &2u64.into() + 1u64.into(),
            )
        };

        let (liquidity, first_token, second_token) = self
            .pair_proxy(pair_address.clone())
            .add_liquidity(
                first_token_amount,
                second_token_amount,
                first_token_amount_min,
                second_token_amount_min,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
            .into_tuple();

        let payment_leftover = if payment_token_id == &first_token.token_id {
            payment_amount_left - &first_token.amount
        } else {
            payment_amount_left - &second_token.amount
        };

        (liquidity, payment_leftover)
    }

    #[endpoint(addIntermediatedPair)]
    #[allow(clippy::too_many_arguments)]
    fn add_intermediated_pair(
        &self,
        address: Address,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
        lp_token_id: TokenIdentifier,
        total_fee_percent: u64,
        special_fee_percent: u64,
        fee_base_points: u64,
    ) -> SCResult<()> {
        only_owner!(self, "denied");
        self.intermediated_pairs().insert(
            address,
            PairContractImmutableInfo {
                token_pair: TokenPair {
                    first_token,
                    second_token,
                },
                lp_token_id,
                total_fee_percent,
                special_fee_percent,
                fee_base_points,
            },
        );
        Ok(())
    }

    #[endpoint(removeIntermediatedPair)]
    fn remove_intermediated_pair(&self, address: Address) -> SCResult<()> {
        only_owner!(self, "denied");
        self.intermediated_pairs().remove(&address);
        Ok(())
    }

    #[storage_mapper("intermediated_pairs")]
    fn intermediated_pairs(&self) -> MapMapper<Self::Storage, Address, PairContractImmutableInfo>;
}
