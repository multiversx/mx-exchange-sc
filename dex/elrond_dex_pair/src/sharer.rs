elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::liquidity_pool;
use multitransfer::EsdtTokenPayment;

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
    + multitransfer::MultiTransferModule
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
        if args.is_empty() {
            return Ok(());
        }

        let my_liquidity = self.liquidity().get();
        if my_liquidity == 0 {
            return Ok(());
        }

        let mut addresses = Vec::new();
        let mut liquidities = Vec::new();
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

        let mut total_liq_needed = Self::BigUint::zero();
        for liq in liquidities.iter() {
            if liq < &avg_liq {
                total_liq_needed += &(&avg_liq - liq);
            }
        }

        let mut liq_transfers = Vec::new();
        let mut transfers = Vec::<Vec<EsdtTokenPayment<Self::BigUint>>>::new();
        let mut transfers_addresses = Vec::new();
        for (index, liq) in liquidities.iter().enumerate() {
            if liq < &avg_liq {
                let liq_needed = &avg_liq - liq;
                let liq_needed_percent = &(&liq_needed * &bp) / &total_liq_needed;
                let liq_amount = &(&liq_to_share * &liq_needed_percent) / &bp;
                let first_token_amount = &(&first_token_to_share * &liq_needed_percent) / &bp;
                let second_token_amount = &(&second_token_to_share * &liq_needed_percent) / &bp;

                if liq_amount != 0 && (first_token_amount != 0 || second_token_amount != 0) {
                    let mut multitransfers = Vec::new();

                    if first_token_amount != 0 {
                        let first_token_transfer = EsdtTokenPayment {
                            token_name: first_token_id.clone(),
                            token_nonce: 0,
                            token_type: EsdtTokenType::Fungible,
                            amount: first_token_amount,
                        };
                        multitransfers.push(first_token_transfer);
                    }

                    if second_token_amount != 0 {
                        let second_token_transfer = EsdtTokenPayment {
                            token_name: second_token_id.clone(),
                            token_nonce: 0,
                            token_type: EsdtTokenType::Fungible,
                            amount: second_token_amount,
                        };
                        multitransfers.push(second_token_transfer);
                    }

                    liq_transfers.push(liq_amount);
                    transfers.push(multitransfers);
                    transfers_addresses.push(addresses[index].clone());
                }
            }
        }

        let endpoint = BoxedBytes::from(&b"acceptLiquidity"[..]);
        for (index, transfer) in transfers.iter().enumerate() {
            let liquidity = &liq_transfers[index];
            let address = &transfers_addresses[index];
            let arg = self.boxed_bytes_from_biguint(&liquidity);

            self.multi_transfer_via_async_call(
                address,
                transfer,
                &endpoint,
                &[arg],
                &BoxedBytes::empty(),
                &[],
            )
        }

        self.liquidity().set(&(my_liquidity - liq_to_share));
        self.update_reserves(
            &(first_token_reserves - first_token_to_share),
            &(second_token_reserves - second_token_to_share),
            &first_token_id,
            &second_token_id,
        );
        Ok(())
    }

    fn boxed_bytes_from_biguint(&self, biguint: &Self::BigUint) -> BoxedBytes {
        BoxedBytes::from(biguint.to_bytes_be().as_slice())
    }

    #[payable("*")]
    #[endpoint(acceptLiquidity)]
    fn accept_liquidity(&self, liquidity: Self::BigUint) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        require!(self.clones().contains(&caller), "Unauthorised caller");
        self.liquidity().update(|x| *x += liquidity);

        let first_token_id = self.first_token_id().get();
        let second_token_id = self.second_token_id().get();
        let transfers = self.get_all_esdt_transfers();
        require!(
            !transfers.is_empty() && transfers.len() <= 2,
            "Wrong payments len"
        );

        for transfer in transfers.iter() {
            require!(
                transfer.token_name == first_token_id || transfer.token_name == second_token_id,
                "Bad transfer token id"
            );
            self.pair_reserve(&transfer.token_name)
                .update(|x| *x += &transfer.amount);
        }
        Ok(())
    }
}
