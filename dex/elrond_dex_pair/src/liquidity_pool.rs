elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::amm;
use super::config;
use common_structs::{FftTokenAmountPair, Nonce};

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
        let total_supply = self.get_total_lp_token_supply();
        let mut first_token_reserve = self.pair_reserve(&first_token).get();
        let mut second_token_reserve = self.pair_reserve(&second_token).get();
        let mut liquidity: Self::BigUint;

        if total_supply == 0 {
            liquidity = core::cmp::min(first_token_amount.clone(), second_token_amount.clone());
            let minimum_liquidity = MINIMUM_LIQUIDITY.into();
            require!(
                liquidity > minimum_liquidity,
                "First tokens needs to be greater than minimum liquidity"
            );
            liquidity -= &minimum_liquidity;
            self.mint_tokens(&self.lp_token_identifier().get(), &minimum_liquidity);
        } else {
            liquidity = core::cmp::min(
                (&first_token_amount * &total_supply) / first_token_reserve.clone(),
                (&second_token_amount * &total_supply) / second_token_reserve.clone(),
            );
        }
        require!(liquidity > 0, "Insufficient liquidity minted");

        first_token_reserve += first_token_amount;
        second_token_reserve += second_token_amount;
        self.set_reserves(
            &first_token,
            &second_token,
            &first_token_reserve,
            &second_token_reserve,
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
        let amount = (liquidity * &reserve) / total_supply.clone();
        require!(amount > 0, "Insufficient liquidity burned");
        require!(&amount >= amount_min, "Insufficient liquidity burned");
        require!(reserve > amount, "Not enough reserve");

        reserve -= &amount;
        self.pair_reserve(token).set(&reserve);

        Ok(amount)
    }

    fn pool_remove_liquidity(
        &self,
        liquidity: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<(Self::BigUint, Self::BigUint)> {
        let total_supply = self.get_total_lp_token_supply();
        require!(
            total_supply >= &liquidity + &MINIMUM_LIQUIDITY.into(),
            "Not enough LP token supply"
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
        let first_token_reserve = self.pair_reserve(&self.first_token_id().get()).get();
        let second_token_reserve = self.pair_reserve(&self.second_token_id().get()).get();

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

    fn set_reserves(
        &self,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) {
        self.pair_reserve(first_token).set(first_token_reserve);
        self.pair_reserve(second_token).set(second_token_reserve);
    }

    fn set_virtual_reserves(
        &self,
        token_side_id: &TokenIdentifier,
        first_token: &TokenIdentifier,
        second_token: &TokenIdentifier,
        first_token_reserve: &Self::BigUint,
        second_token_reserve: &Self::BigUint,
    ) {
        self.pair_virtual_reserve(token_side_id, first_token)
            .set(first_token_reserve);
        self.pair_virtual_reserve(token_side_id, second_token)
            .set(second_token_reserve);
    }

    fn increase_token_reserve(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) {
        self.pair_reserve(token_id)
            .update(|reserve| *reserve += amount);
    }

    fn try_decrease_token_reserve(
        &self,
        token_id: &TokenIdentifier,
        amount: &Self::BigUint,
    ) -> SCResult<()> {
        self.pair_reserve(token_id).update(|reserve| {
            require!(&*reserve > amount, "Not enough reserves");
            *reserve -= amount;
            Ok(())
        })
    }

    fn decrease_token_reserve(&self, token_id: &TokenIdentifier, amount: &Self::BigUint) {
        self.pair_reserve(token_id)
            .update(|reserve| *reserve -= amount);
    }

    fn get_token_for_given_position(
        &self,
        liquidity: Self::BigUint,
        token_id: TokenIdentifier,
    ) -> FftTokenAmountPair<Self::BigUint> {
        let reserve = self.pair_reserve(&token_id).get();
        let total_supply = self.get_total_lp_token_supply();
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
        let first_token_amount = self.pair_reserve(&self.first_token_id().get()).get();
        let second_token_amount = self.pair_reserve(&self.second_token_id().get()).get();
        self.calculate_k_constant(&first_token_amount, &second_token_amount)
    }

    fn calculate_k_for_virtual_reserves(&self, token_side_id: &TokenIdentifier) -> Self::BigUint {
        let first_token_amount = self
            .pair_virtual_reserve(token_side_id, &self.first_token_id().get())
            .get();
        let second_token_amount = self
            .pair_virtual_reserve(token_side_id, &self.second_token_id().get())
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
        let big_zero = Self::BigUint::zero();
        let first_token_reserve = self.pair_virtual_reserve(token_in, first_token_id).get();
        let second_token_reserve = self.pair_virtual_reserve(token_in, second_token_id).get();

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

        let amount_out = self.get_amount_out_no_fee(amount_in, &reserve_in, &reserve_out);
        if reserve_out <= amount_out || amount_out == 0 {
            return big_zero;
        }

        if self.pair_reserve(token_out).get() <= amount_out {
            return big_zero;
        }

        reserve_in += amount_in;
        reserve_out -= &amount_out;
        self.set_virtual_reserves(token_in, token_in, token_out, &reserve_in, &reserve_out);
        self.increase_token_reserve(token_in, amount_in);
        self.decrease_token_reserve(token_out, &amount_out);

        amount_out
    }

    fn update_virtual_reserves_on_block_change(&self) {
        let current_block_nonce = self.blockchain().get_block_nonce();

        if current_block_nonce > self.last_recorded_block_nonce().get() {
            self.last_recorded_block_nonce().set(&current_block_nonce);

            let first_token_id = self.first_token_id().get();
            let second_token_id = self.second_token_id().get();

            let first_token_reserve = self.pair_reserve(&first_token_id).get();
            let second_token_reserve = self.pair_reserve(&second_token_id).get();

            self.pair_virtual_reserve(&first_token_id, &first_token_id)
                .set(&first_token_reserve);
            self.pair_virtual_reserve(&second_token_id, &first_token_id)
                .set(&first_token_reserve);
            self.pair_virtual_reserve(&first_token_id, &second_token_id)
                .set(&second_token_reserve);
            self.pair_virtual_reserve(&second_token_id, &second_token_id)
                .set(&second_token_reserve);
        }
    }

    fn get_reserves_for_current_block(
        &self,
        token_side_id: &TokenIdentifier,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
    ) -> (Self::BigUint, Self::BigUint) {
        let last_recorded_block_nonce = self.last_recorded_block_nonce().get();

        if last_recorded_block_nonce == self.blockchain().get_block_nonce() {
            (
                self.pair_virtual_reserve(token_side_id, first_token_id)
                    .get(),
                self.pair_virtual_reserve(token_side_id, second_token_id)
                    .get(),
            )
        } else {
            (
                self.pair_reserve(first_token_id).get(),
                self.pair_reserve(second_token_id).get(),
            )
        }
    }

    #[view(getTotalSupply)]
    fn get_total_lp_token_supply(&self) -> Self::BigUint {
        let result = self.get_total_supply(&self.lp_token_identifier().get());
        match result {
            SCResult::Ok(amount) => amount,
            SCResult::Err(message) => self.send().signal_error(message.as_bytes()),
        }
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

    #[view(getLastRecordedBlockNonce)]
    #[storage_mapper("last_recorded_block_nonce")]
    fn last_recorded_block_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[view(getVirtualReserve)]
    #[storage_mapper("virtual_reserve")]
    fn pair_virtual_reserve(
        &self,
        token_side_id: &TokenIdentifier,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, Self::BigUint>;
}
