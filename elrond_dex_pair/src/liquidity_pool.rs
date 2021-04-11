imports!();
derive_imports!();

pub use crate::amm::*;

const MINIMUM_LIQUIDITY: u64 = 1000;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct TokenAmountPair<BigUint: BigUintApi> {
    token_id: TokenIdentifier,
    amount: BigUint,
}

#[elrond_wasm_derive::module(LiquidityPoolModuleImpl)]
pub trait LiquidityPoolModule {
    #[module(AmmModuleImpl)]
    fn amm(&self) -> AmmModuleImpl<T, BigInt, BigUint>;

    fn mint(
        &self,
        first_token_amount: BigUint,
        second_token_amount: BigUint,
        lp_token_identifier: TokenIdentifier,
    ) -> SCResult<BigUint> {
        let first_token = self.first_token_id().get();
        let second_token = self.second_token_id().get();
        let mut total_supply = self.total_supply().get();
        let mut first_token_reserve = self.get_pair_reserve(&first_token);
        let mut second_token_reserve = self.get_pair_reserve(&second_token);
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
            self.get_gas_left(),
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
        let mut reserve = self.get_pair_reserve(&token);
        let amount = (liquidity * reserve.clone()) / total_supply;
        require!(amount > 0, "Pair: insufficient_liquidity_burned");
        require!(amount >= amount_min, "Pair: insufficient_liquidity_burned");
        require!(reserve > amount, "Not enough reserve");

        reserve -= amount.clone();
        self.set_pair_reserve(&token, &reserve);

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
        require!(total_supply > 0, "No supply");
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
            self.get_gas_left(),
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
        let first_token_reserve = self.get_pair_reserve(&self.first_token_id().get());
        let second_token_reserve = self.get_pair_reserve(&self.second_token_id().get());

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
                "Pair: insufficient_b_amount"
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
                "Pair: insufficient_a_amount"
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
        self.set_pair_reserve(first_token, first_token_reserve);
        self.set_pair_reserve(second_token, second_token_reserve);
    }

    fn get_token_for_given_position(
        &self,
        liquidity: BigUint,
        token_id: TokenIdentifier,
    ) -> TokenAmountPair<BigUint> {
        let reserve = self.get_pair_reserve(&token_id);
        let total_supply = self.total_supply().get();
        if total_supply != BigUint::zero() {
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
        let first_token_amount = self.get_pair_reserve(&self.first_token_id().get());
        let second_token_amount = self.get_pair_reserve(&self.second_token_id().get());
        self.amm()
            .calculate_k_constant(first_token_amount, second_token_amount)
    }

    #[view(getFirstTokenId)]
    #[storage_mapper("first_token_id")]
    fn first_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getSecondTokenId)]
    #[storage_mapper("second_token_id")]
    fn second_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getReserve)]
    #[storage_get("reserve")]
    fn get_pair_reserve(&self, token: &TokenIdentifier) -> BigUint;

    #[storage_set("reserve")]
    fn set_pair_reserve(&self, token: &TokenIdentifier, reserve: &BigUint);

    #[storage_clear("reserve")]
    fn clear_pair_reserve(&self, token: &TokenIdentifier);

    #[view(getTotalSupply)]
    #[storage_mapper("total_supply")]
    fn total_supply(&self) -> SingleValueMapper<Self::Storage, BigUint>;
}
