elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::liquidity_pool;

use super::amm;
use super::config;

#[elrond_wasm::module]
pub trait SharerModule:
    info_sync::InfoSyncModule
    + amm::AmmModule
    + config::ConfigModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + liquidity_pool::LiquidityPoolModule
{
    #[endpoint(shareInformation)]
    fn share_information(&self) -> SCResult<()> {
        let total_liquidity = self.liquidity().get();
        let block = self.blockchain().get_block_nonce();

        if block > self.last_info_share_block().get() + self.info_share_min_blocks().get() {
            self.last_info_share_block().set(&block);
            self.broadcast_information(total_liquidity.to_bytes_be().into())?;
        }
        Ok(())
    }

    #[endpoint(takeActionOnInformationReceive)]
    fn take_action_on_information_receive(
        &self,
        #[var_args] args: MultiArgVec<MultiArg2<Address, BoxedBytes>>,
    ) -> SCResult<()> {
        let my_liquidity = self.liquidity().get();
        if my_liquidity == 0 {
            return Ok(());
        }

        let mut addresses = Vec::new();
        let mut liquidities = Vec::new();
        if args.is_empty() {
            return Ok(());
        }

        for arg in args.into_vec() {
            let tuple = &arg.0;
            addresses.push(tuple.0.clone());
            liquidities.push(Self::BigUint::from_bytes_be(tuple.1.as_slice()));
        }

        let mut liq_sum = Self::BigUint::zero();
        for liq in liquidities.iter() {
            liq_sum += liq;
        }

        if liq_sum == 0 {
            return Ok(());
        }

        if my_liquidity > &liq_sum / &liquidities.len().into() {
            self.send_liquidity(my_liquidity, liq_sum, addresses, liquidities)
        } else {
            Ok(())
        }
    }

    fn send_liquidity(
        &self,
        my_liquidity: Self::BigUint,
        liq_sum: Self::BigUint,
        addresses: Vec<Address>,
        liquidities: Vec<Self::BigUint>,
    ) -> SCResult<()> {
        let avg_liq = &liq_sum / &liquidities.len().into();
        let liq_to_share = &avg_liq - &my_liquidity;
        if liq_to_share == 0 {
            return Ok(());
        }

        let bp = Self::BigUint::from(100_000u64);
        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let first_token_reserves = self.pair_reserve(&first_token_id).get();
        let second_token_reserves = self.pair_reserve(&second_token_id).get();

        let liq_to_share_percent = &(&liq_to_share * &bp) / &my_liquidity;
        let first_token_to_share = &(&first_token_reserves * &liq_to_share_percent) / &bp;
        let second_token_to_share = &(&second_token_reserves * &liq_to_share_percent) / &bp;
        if first_token_to_share == 0 || second_token_to_share == 0 {
            return Ok(());
        }

        //TODO: Figure out what amount to send and to whom and then send

        self.liquidity().set(&(my_liquidity - liq_to_share));
        self.update_reserves(
            &(first_token_reserves - first_token_to_share),
            &(second_token_reserves - second_token_to_share),
            &first_token_id,
            &second_token_id,
        );
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptLiquidity)]
    fn accept_liquidity(&self, liquidity: Self::BigUint) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(self.clones().contains(&caller), "Unauthorised caller");
        self.liquidity().update(|x| *x += liquidity);

        let payment_first_token = TokenIdentifier::egld();
        let payment_second_token = TokenIdentifier::egld();
        let payment_first_amount = Self::BigUint::zero();
        let payment_second_amount = Self::BigUint::zero();

        let first_token_reserve =
            self.pair_reserve(&payment_first_token).get() + payment_first_amount;
        let second_token_reserve =
            self.pair_reserve(&payment_second_token).get() + payment_second_amount;
        self.update_reserves(
            &first_token_reserve,
            &second_token_reserve,
            &payment_first_token,
            &payment_second_token,
        );
        Ok(())
    }
}
