elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub use crate::amm::*;
pub use crate::fee::*;

const MINIMUM_LIQUIDITY: u64 = 1000;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct TokenAmountPair<BigUint: BigUintApi> {
    pub token_id: TokenIdentifier,
    pub amount: BigUint,
}

#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {
    #[module(AmmModuleImpl)]
    fn amm(&self) -> AmmModuleImpl<T, BigInt, BigUint>;

    #[module(FeeModuleImpl)]
    fn fee(&self) -> FeeModuleImpl<T, BigInt, BigUint>;

    fn mint(
        &self,
        first_token_amount: BigUint,
        second_token_amount: BigUint,
        lp_token_identifier: TokenIdentifier,
    ) -> SCResult<BigUint> {
        let first_token = self.first_token_id().get();
        let second_token = self.second_token_id().get();
        let mut total_supply = self.total_supply().get();
        let mut first_token_reserve = self.pair_reserve(&first_token).get();
        let mut second_token_reserve = self.pair_reserve(&second_token).get();
        let mut liquidity: BigUint;

        if total_supply == 0 {
            liquidity = core::cmp::min(first_token_amount.clone(), second_token_amount.clone());
            let minimum_liquidity = BigUint::from(MINIMUM_LIQUIDITY);
            require!(
                liquidity > minimum_liquidity,
                "Pair: first tokens needs to be grater than minimum liquidity"
            );
            liquidity -= minimum_liquidity.clone();
            total_supply += minimum_liquidity;
            self.total_supply().set(&total_supply);
        } else {
            liquidity = core::cmp::min(
                (first_token_amount.clone() * total_supply.clone()) / first_token_reserve.clone(),
                (second_token_amount.clone() * total_supply) / second_token_reserve.clone(),
            );
        }

        require!(liquidity > 0, "Pair: insufficient_liquidity_minted");

        self.send().esdt_local_mint(
            self.blockchain().get_gas_left(),
            lp_token_identifier.as_esdt_identifier(),
            &liquidity,
        );

        first_token_reserve += first_token_amount;
        second_token_reserve += second_token_amount;
        self.update_reserves(
            &first_token_reserve,
            &second_token_reserve,
            &first_token,
            &second_token,
        );

        Ok(liquidity)
    }

    fn burn_token(
        &self,
        token: TokenIdentifier,
        liquidity: BigUint,
        total_supply: BigUint,
        amount_min: BigUint,
    ) -> SCResult<BigUint> {
        let mut reserve = self.pair_reserve(&token).get();
        let amount = (liquidity * reserve.clone()) / total_supply;
        require!(amount > 0, "Pair: insufficient_liquidity_burned");
        require!(amount >= amount_min, "Pair: insufficient_liquidity_burned");
        require!(reserve > amount, "Not enough reserve");

        reserve -= amount.clone();
        self.pair_reserve(&token).set(&reserve);

        Ok(amount)
    }

    fn burn(
        &self,
        liquidity: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        lp_token_identifier: TokenIdentifier,
    ) -> SCResult<(BigUint, BigUint)> {
        let total_supply = self.total_supply().get();
        require!(total_supply > 0, "No LP tokens supply");
        let first_token_amount = sc_try!(self.burn_token(
            self.first_token_id().get(),
            liquidity.clone(),
            total_supply.clone(),
            first_token_amount_min
        ));
        let second_token_amount = sc_try!(self.burn_token(
            self.second_token_id().get(),
            liquidity.clone(),
            total_supply,
            second_token_amount_min
        ));

        self.send().esdt_local_burn(
            self.blockchain().get_gas_left(),
            lp_token_identifier.as_esdt_identifier(),
            &liquidity,
        );

        Ok((first_token_amount, second_token_amount))
    }

    fn add_liquidity(
        &self,
        first_token_amount_desired: BigUint,
        second_token_amount_desired: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> SCResult<(BigUint, BigUint)> {
        let first_token_reserve = self.pair_reserve(&self.first_token_id().get()).get();
        let second_token_reserve = self.pair_reserve(&self.second_token_id().get()).get();

        if first_token_reserve == 0 && second_token_reserve == 0 {
            return Ok((first_token_amount_desired, second_token_amount_desired));
        }

        let second_token_amount_optimal = self.amm().quote(
            first_token_amount_desired.clone(),
            first_token_reserve.clone(),
            second_token_reserve.clone(),
        );
        if second_token_amount_optimal <= second_token_amount_desired {
            require!(
                second_token_amount_optimal > second_token_amount_min,
                "Pair: insufficient second token computed amount"
            );
            Ok((first_token_amount_desired, second_token_amount_optimal))
        } else {
            let first_token_amount_optimal = self.amm().quote(
                second_token_amount_desired.clone(),
                second_token_reserve,
                first_token_reserve,
            );
            require!(
                first_token_amount_optimal <= first_token_amount_desired,
                "Pair: optimal amount grater than desired amount"
            );
            require!(
                first_token_amount_optimal >= first_token_amount_min,
                "Pair: insufficient first token computed amount"
            );
            Ok((first_token_amount_optimal, second_token_amount_desired))
        }
    }

    fn update_reserves(
        &self,
        first_token_reserve: &BigUint,
        second_token_reserve: &BigUint,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) {
        self.pair_reserve(first_token).set(first_token_reserve);
        self.pair_reserve(second_token).set(second_token_reserve);
    }

    fn get_token_for_given_position(
        &self,
        liquidity: BigUint,
        token_id: TokenIdentifier,
    ) -> TokenAmountPair<BigUint> {
        let reserve = self.pair_reserve(&token_id).get();
        let total_supply = self.total_supply().get();
        if total_supply != 0 {
            TokenAmountPair {
                token_id,
                amount: liquidity * reserve / total_supply,
            }
        } else {
            TokenAmountPair {
                token_id,
                amount: BigUint::zero(),
            }
        }
    }

    fn get_both_tokens_for_given_position(
        &self,
        liquidity: BigUint,
    ) -> MultiResult2<TokenAmountPair<BigUint>, TokenAmountPair<BigUint>> {
        let first_token_id = self.first_token_id().get();
        let token_first_token_amount =
            self.get_token_for_given_position(liquidity.clone(), first_token_id);
        let second_token_id = self.second_token_id().get();
        let token_second_token_amount =
            self.get_token_for_given_position(liquidity, second_token_id);
        (token_first_token_amount, token_second_token_amount).into()
    }

    fn calculate_k_for_reserves(&self) -> BigUint {
        let first_token_amount = self.pair_reserve(&self.first_token_id().get()).get();
        let second_token_amount = self.pair_reserve(&self.second_token_id().get()).get();
        self.amm()
            .calculate_k_constant(first_token_amount, second_token_amount)
    }

    fn swap_safe_no_fee(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        token_in: &TokenIdentifier,
        amount_in: &BigUint,
    ) -> BigUint {
        let big_zero = BigUint::zero();
        let first_token_reserve = self.pair_reserve(first_token_id).get();
        let second_token_reserve = self.pair_reserve(second_token_id).get();

        let (token_in, mut reserve_in, token_out, mut reserve_out) = if token_in == first_token_id {
            (
                first_token_id,
                first_token_reserve,
                second_token_id,
                second_token_reserve,
            )
        } else {
            (
                second_token_id,
                second_token_reserve,
                first_token_id,
                first_token_reserve,
            )
        };

        if reserve_out == 0 {
            return big_zero;
        }

        let amount_out = self.amm().get_amount_out_no_fee(
            amount_in.clone(),
            reserve_in.clone(),
            reserve_out.clone(),
        );

        if reserve_out <= amount_out {
            return big_zero;
        }

        reserve_in += amount_in;
        reserve_out -= amount_out.clone();
        self.update_reserves(&reserve_in, &reserve_out, &token_in, &token_out);

        amount_out
    }

    #[view(getFirstTokenId)]
    #[storage_mapper("first_token_id")]
    fn first_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getSecondTokenId)]
    #[storage_mapper("second_token_id")]
    fn second_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getReserve)]
    #[storage_mapper("reserve")]
    fn pair_reserve(&self, token_id: &TokenIdentifier)
        -> SingleValueMapper<Self::Storage, BigUint>;

    #[view(getTotalSupply)]
    #[storage_mapper("total_supply")]
    fn total_supply(&self) -> SingleValueMapper<Self::Storage, BigUint>;
}
