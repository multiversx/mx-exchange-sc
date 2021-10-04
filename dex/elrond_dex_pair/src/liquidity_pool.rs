elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::amm;
use super::config;
use common_structs::FftTokenAmountPair;

const MINIMUM_LIQUIDITY: u64 = 1_000;

#[elrond_wasm::module]
pub trait LiquidityPoolModule:
    amm::AmmModule
    + config::ConfigModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
{
    fn pool_add_liquidity(
        &self,
        first_token_amount: Self::BigUint,
        second_token_amount: Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        let first_token = self.first_token_id().get();
        let second_token = self.second_token_id().get();
        let total_virtual_supply = self.virtual_liquitiy().get();
        let mut first_token_reserve = self.pair_reserve(&first_token).get();
        let mut second_token_reserve = self.pair_reserve(&second_token).get();
        let mut first_token_virtual_reserve = self.pair_virtual_reserve(&first_token).get();
        let mut second_token_virtual_reserve = self.pair_virtual_reserve(&second_token).get();
        let mut liquidity: Self::BigUint;

        if total_virtual_supply == 0 {
            liquidity = core::cmp::min(first_token_amount.clone(), second_token_amount.clone());
            let minimum_liquidity = MINIMUM_LIQUIDITY.into();
            require!(
                liquidity > minimum_liquidity,
                "First tokens needs to be greater than minimum liquidity"
            );
            liquidity -= &minimum_liquidity;
            self.liquidity().set(&minimum_liquidity);
            self.virtual_liquitiy().set(&minimum_liquidity);
            self.mint_tokens(&self.lp_token_identifier().get(), &minimum_liquidity);
        } else {
            liquidity = core::cmp::min(
                (&first_token_amount * &total_virtual_supply) / first_token_virtual_reserve.clone(),
                (&second_token_amount * &total_virtual_supply)
                    / second_token_virtual_reserve.clone(),
            );
        }
        require!(liquidity > 0, "Insufficient liquidity minted");

        first_token_reserve += &first_token_amount;
        second_token_reserve += &second_token_amount;
        self.update_reserves(
            &first_token_reserve,
            &second_token_reserve,
            &first_token,
            &second_token,
        );

        first_token_virtual_reserve += &first_token_amount;
        second_token_virtual_reserve += &second_token_amount;
        self.update_virtual_reserves(
            &first_token_virtual_reserve,
            &second_token_virtual_reserve,
            &first_token,
            &second_token,
        );

        Ok(liquidity)
    }

    fn remove_token(
        &self,
        token: &TokenIdentifier,
        liquidity: &Self::BigUint,
        total_supply: &Self::BigUint,
        amount_min: &Self::BigUint,
    ) -> SCResult<Self::BigUint> {
        let mut reserve = self.pair_reserve(token).get();
        let mut virtual_reserve = self.pair_virtual_reserve(token).get();
        let amount = (liquidity * &virtual_reserve) / total_supply.clone();
        require!(amount > 0, "Insufficient liquidity burned");
        require!(&amount >= amount_min, "Slippage amount does not match");
        require!(virtual_reserve > amount, "Not enough virtual reserve");
        require!(reserve > amount, "Not enough reserve");

        reserve -= &amount;
        self.pair_reserve(token).set(&reserve);

        virtual_reserve -= &amount;
        self.pair_virtual_reserve(token).set(&virtual_reserve);

        Ok(amount)
    }

    fn pool_remove_liquidity(
        &self,
        liquidity: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<(Self::BigUint, Self::BigUint)> {
        let total_supply = self.virtual_liquitiy().get();
        require!(
            total_supply >= &liquidity + &MINIMUM_LIQUIDITY.into(),
            "Not enough LP token virtual supply"
        );

        let first_token_amount = self.remove_token(
            &self.first_token_id().get(),
            &liquidity,
            &total_supply,
            &first_token_amount_min,
        )?;
        let second_token_amount = self.remove_token(
            &self.second_token_id().get(),
            &liquidity,
            &total_supply,
            &second_token_amount_min,
        )?;

        Ok((first_token_amount, second_token_amount))
    }

    fn calculate_optimal_amounts(
        &self,
        first_token_amount_desired: Self::BigUint,
        second_token_amount_desired: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<(Self::BigUint, Self::BigUint)> {
        let first_token_reserve = self
            .pair_virtual_reserve(&self.first_token_id().get())
            .get();
        let second_token_reserve = self
            .pair_virtual_reserve(&self.second_token_id().get())
            .get();

        if first_token_reserve == 0 && second_token_reserve == 0 {
            return Ok((first_token_amount_desired, second_token_amount_desired));
        }

        let second_token_amount_optimal = self.quote(
            &first_token_amount_desired,
            &first_token_reserve,
            &second_token_reserve,
        );
        if second_token_amount_optimal <= second_token_amount_desired {
            require!(
                second_token_amount_optimal >= second_token_amount_min,
                "Insufficient second token computed amount"
            );
            Ok((first_token_amount_desired, second_token_amount_optimal))
        } else {
            let first_token_amount_optimal = self.quote(
                &second_token_amount_desired,
                &second_token_reserve,
                &first_token_reserve,
            );
            require!(
                first_token_amount_optimal <= first_token_amount_desired,
                "Optimal amount greater than desired amount"
            );
            require!(
                first_token_amount_optimal >= first_token_amount_min,
                "Insufficient first token computed amount"
            );
            Ok((first_token_amount_optimal, second_token_amount_desired))
        }
    }

    fn update_reserves(
        &self,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) {
        self.pair_reserve(first_token).set(first_token_reserve);
        self.pair_reserve(second_token).set(second_token_reserve);
    }

    fn update_virtual_reserves(
        &self,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
    ) {
        self.pair_virtual_reserve(first_token)
            .set(first_token_reserve);
        self.pair_virtual_reserve(second_token)
            .set(second_token_reserve);
    }

    fn get_token_for_given_position(
        &self,
        liquidity: Self::BigUint,
        token_id: TokenIdentifier,
    ) -> FftTokenAmountPair<Self::BigUint> {
        let reserve = self.pair_reserve(&token_id).get();
        let total_supply = self.liquidity().get();
        if total_supply != 0 {
            FftTokenAmountPair {
                token_id,
                amount: liquidity * reserve / total_supply,
            }
        } else {
            FftTokenAmountPair {
                token_id,
                amount: 0u64.into(),
            }
        }
    }

    fn get_both_tokens_for_given_position(
        &self,
        liquidity: Self::BigUint,
    ) -> MultiResult2<FftTokenAmountPair<Self::BigUint>, FftTokenAmountPair<Self::BigUint>> {
        let first_token_id = self.first_token_id().get();
        let token_first_token_amount =
            self.get_token_for_given_position(liquidity.clone(), first_token_id);
        let second_token_id = self.second_token_id().get();
        let token_second_token_amount =
            self.get_token_for_given_position(liquidity, second_token_id);
        (token_first_token_amount, token_second_token_amount).into()
    }

    fn calculate_k_for_reserves(&self) -> Self::BigUint {
        let first_token_amount = self
            .pair_virtual_reserve(&self.first_token_id().get())
            .get();
        let second_token_amount = self
            .pair_virtual_reserve(&self.second_token_id().get())
            .get();
        self.calculate_k_constant(&first_token_amount, &second_token_amount)
    }

    fn swap_safe_no_fee(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
        token_in: &TokenIdentifier,
        amount_in: &Self::BigUint,
    ) -> Self::BigUint {
        let big_zero = 0u64.into();
        let first_token_reserve = self.pair_reserve(first_token_id).get();
        let second_token_reserve = self.pair_reserve(second_token_id).get();
        let first_token_virtual_reserve = self.pair_virtual_reserve(first_token_id).get();
        let second_token_virtual_reserve = self.pair_virtual_reserve(second_token_id).get();

        let (
            token_in,
            mut reserve_in,
            mut virtual_reserve_in,
            token_out,
            mut reserve_out,
            mut virtual_reserve_out,
        ) = if token_in == first_token_id {
            (
                first_token_id,
                first_token_reserve,
                first_token_virtual_reserve,
                second_token_id,
                second_token_reserve,
                second_token_virtual_reserve,
            )
        } else {
            (
                second_token_id,
                second_token_reserve,
                second_token_virtual_reserve,
                first_token_id,
                first_token_reserve,
                first_token_virtual_reserve,
            )
        };

        if reserve_out == 0 {
            return big_zero;
        }

        let amount_out =
            self.get_amount_out_no_fee(amount_in, &virtual_reserve_in, &virtual_reserve_out);
        if virtual_reserve_out <= amount_out && reserve_out <= amount_out || amount_out == 0 {
            return big_zero;
        }

        reserve_in += amount_in;
        reserve_out -= &amount_out;
        self.update_reserves(&reserve_in, &reserve_out, token_in, token_out);

        virtual_reserve_in += amount_in;
        virtual_reserve_out -= &amount_out;
        self.update_virtual_reserves(
            &virtual_reserve_in,
            &virtual_reserve_out,
            token_in,
            token_out,
        );

        amount_out
    }

    fn local_and_virtual_price_differ_too_much(&self) -> bool {
        let price_bp = Self::BigUint::from(100_000_000u64);
        let price_threshold_percent = 10u64;
        let price_percent_total = 100u64;
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();

        let first_token_reserve_local = self.pair_reserve(&first_token_id).get();
        let second_token_reserve_local = self.pair_reserve(&second_token_id).get();
        let first_token_reserve_virtual = self.pair_virtual_reserve(&first_token_id).get();
        let second_token_reserve_virtual = self.pair_virtual_reserve(&second_token_id).get();

        let first_token_price_local =
            first_token_reserve_local * price_bp.clone() / second_token_reserve_local;

        let first_token_price_virtual =
            first_token_reserve_virtual * price_bp / second_token_reserve_virtual;

        let first_token_price_virtual_min = first_token_price_virtual.clone()
            * (price_percent_total - price_threshold_percent).into()
            / price_percent_total.into();

        let first_token_price_virtual_max = first_token_price_virtual
            * (price_percent_total + price_threshold_percent).into()
            / price_percent_total.into();

        first_token_price_local > first_token_price_virtual_min
            && first_token_price_local < first_token_price_virtual_max
    }

    fn swap_too_big(&self, amount: &Self::BigUint, reserve: &Self::BigUint) -> bool {
        amount > &(reserve / &10u64.into())
    }

    #[view(getFirstTokenId)]
    #[storage_mapper("first_token_id")]
    fn first_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getSecondTokenId)]
    #[storage_mapper("second_token_id")]
    fn second_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getReserve)]
    #[storage_mapper("reserve")]
    fn pair_reserve(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getVirtualReserve)]
    #[storage_mapper("virtual_reserve")]
    fn pair_virtual_reserve(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getLiquidity)]
    #[storage_mapper("liquidity")]
    fn liquidity(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getVirtualLiquidity)]
    #[storage_mapper("virtual_liquitiy")]
    fn virtual_liquitiy(&self) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
