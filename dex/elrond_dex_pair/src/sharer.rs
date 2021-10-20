elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::liquidity_pool;
use elrond_wasm::elrond_codec::TopEncode;
use multitransfer::EsdtTokenPayment;

use super::amm;
use super::config;
use super::safe_reserves;

const BP: u64 = 100_000;
const GAS_COST_FOR_SEND_LIQUIDITY: u64 = 100_000_000u64;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct SharedInformation<BigUint: BigUintApi> {
    pub metadata: InformationMetadata,
    pub liquidity_info: LiquidityInformation<BigUint>,
    pub swap_stats: SwapStatistics<BigUint>,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct InformationMetadata {
    pub sender: Address,
    pub timestamp: u64,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct LiquidityInformation<BigUint: BigUintApi> {
    pub liquidity_amount: BigUint,
    pub first_token_amount: BigUint,
    pub second_token_amount: BigUint,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct SwapStatistics<BigUint: BigUintApi> {
    pub _placeholder: BigUint,
}

impl<BigUint: BigUintApi> SharedInformation<BigUint> {
    pub fn to_boxed_bytes(&self) -> BoxedBytes {
        let mut vec = Vec::new();
        let result = self.top_encode(&mut vec);
        match result {
            Result::Ok(_) => BoxedBytes::from(vec.as_slice()),
            Result::Err(_) => BoxedBytes::empty(),
        }
    }

    pub fn from_boxed_bytes(bytes: BoxedBytes) -> SCResult<SharedInformation<BigUint>> {
        SharedInformation::<BigUint>::top_decode(bytes.as_slice()).into()
    }
}

#[elrond_wasm::module]
pub trait SharerModule:
    info_sync::InfoSyncModule
    + amm::AmmModule
    + config::ConfigModule
    + token_supply::TokenSupplyModule
    + token_send::TokenSendModule
    + liquidity_pool::LiquidityPoolModule
    + multitransfer::MultiTransferModule
    + safe_reserves::SafeReserveModule
{
    #[endpoint(shareInformation)]
    fn share_information(&self) -> SCResult<()> {
        let block = self.blockchain().get_block_nonce();

        if self.min_blocks_passed(block)
            || self.has_received_info()
            || self.local_and_virtual_price_differ_too_much()
        {
            self.last_info_share_block().set(&block);
            let shared_info = self.own_shared_info_get_or_create();
            let shared_info_bytes = shared_info.to_boxed_bytes();
            require!(!shared_info_bytes.is_empty(), "Error encoding");
            self.broadcast_information(shared_info_bytes)?;
            self.own_shared_info_set_if_empty_or_clear(shared_info);
        }
        Ok(())
    }

    #[endpoint(takeActionOnInformationReceive)]
    fn take_action_on_information_receive(&self) -> SCResult<()> {
        let recv_info = self.get_recv_info_decoded()?;
        require!(recv_info.len() == self.clones().len(), "Not enough info");
        let own_info = self.own_shared_info_get_or_create();

        let recv_liquidity_info = self.exteract_liquidity_info(&recv_info);
        let mut all_liquidity_info = recv_liquidity_info;
        all_liquidity_info.push(own_info.liquidity_info.clone());

        let all_liquidity_info_len = Self::BigUint::from(all_liquidity_info.len());
        let all_liquidity_sum = self.compute_sum_all_liquidity(&all_liquidity_info);

        let all_liquidity_avg = LiquidityInformation {
            liquidity_amount: &all_liquidity_sum.liquidity_amount / &all_liquidity_info_len,
            first_token_amount: &all_liquidity_sum.first_token_amount / &all_liquidity_info_len,
            second_token_amount: &all_liquidity_sum.second_token_amount / &all_liquidity_info_len,
        };
        let liquidity_amount_threshold =
            self.compute_liquidity_amount_max_threshold(&all_liquidity_avg.liquidity_amount);

        if own_info.liquidity_info.liquidity_amount > liquidity_amount_threshold {
            self.try_send_liquidity(&own_info, recv_info, all_liquidity_avg)?;
        }

        self.virtual_liquitiy()
            .set(&all_liquidity_sum.liquidity_amount);
        self.pair_virtual_reserve(&self.first_token_id().get())
            .set(&all_liquidity_sum.first_token_amount);
        self.pair_virtual_reserve(&self.second_token_id().get())
            .set(&all_liquidity_sum.second_token_amount);
        self.own_shared_info_set_if_empty_or_clear(own_info);
        Ok(())
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

    fn try_send_liquidity(
        &self,
        own_info: &SharedInformation<Self::BigUint>,
        all_recv_shared_info: Vec<SharedInformation<Self::BigUint>>,
        avg_liquidity_info: LiquidityInformation<Self::BigUint>,
    ) -> SCResult<()> {
        let mut all_liq_needed = Self::BigUint::zero();
        for shared_info in all_recv_shared_info.iter() {
            if shared_info.liquidity_info.liquidity_amount < avg_liquidity_info.liquidity_amount {
                all_liq_needed += &(&avg_liquidity_info.liquidity_amount
                    - &shared_info.liquidity_info.liquidity_amount);
            }
        }

        let all_liq_to_share =
            &own_info.liquidity_info.liquidity_amount - &avg_liquidity_info.liquidity_amount;

        let mut current_liquidity = self.liquidity().get();
        let first_token_id = self.first_token_id().get();
        let mut first_token_reserve = self.pair_reserve(&first_token_id).get();
        let second_token_id = self.second_token_id().get();
        let mut second_token_reserve = self.pair_reserve(&second_token_id).get();

        let mut payments = Vec::new();
        let mut addresses = Vec::new();
        let mut liquidity_amounts = Vec::new();

        for shared_info in all_recv_shared_info.iter() {
            if shared_info.liquidity_info.liquidity_amount < avg_liquidity_info.liquidity_amount {
                let liq_needed = &avg_liquidity_info.liquidity_amount
                    - &shared_info.liquidity_info.liquidity_amount;

                let liq_needed_percent = &(&liq_needed * &BP.into()) / &all_liq_needed;
                let liq_to_share = &(&all_liq_to_share * &liq_needed_percent) / &BP.into();

                if liq_to_share > 0 {
                    require!(current_liquidity > liq_to_share, "not enough liquidity");
                    current_liquidity -= &liq_to_share;

                    let (first, second) = self
                        .get_both_tokens_for_given_position(liq_to_share.clone())
                        .into_tuple();

                    let mut multitransfers = Vec::new();
                    if first.amount > 0 {
                        first_token_reserve -= &first.amount;
                        multitransfers.push(EsdtTokenPayment {
                            token_type: EsdtTokenType::Fungible,
                            amount: first.amount,
                            token_name: first.token_id,
                            token_nonce: 0,
                        });
                    }

                    if second.amount > 0 {
                        second_token_reserve -= &second.amount;
                        multitransfers.push(EsdtTokenPayment {
                            token_type: EsdtTokenType::Fungible,
                            amount: second.amount,
                            token_name: second.token_id,
                            token_nonce: 0,
                        });
                    }

                    payments.push(multitransfers);
                    addresses.push(shared_info.metadata.sender.clone());
                    liquidity_amounts.push(liq_to_share.clone());
                }
            }
        }

        self.liquidity().set(&current_liquidity);
        self.pair_reserve(&first_token_id).set(&first_token_reserve);
        self.pair_reserve(&second_token_id)
            .set(&second_token_reserve);

        let per_transfer_gas_limit = self.blockchain().get_gas_left() / (payments.len() as u64 + 1);
        require!(
            per_transfer_gas_limit > GAS_COST_FOR_SEND_LIQUIDITY,
            "Not enough gas for each transfer"
        );

        let endpoint = BoxedBytes::from(&b"acceptLiquidity"[..]);
        for (index, transfer) in payments.iter().enumerate() {
            let liquidity = &liquidity_amounts[index];
            let address = &addresses[index];
            let arg = self.arg_buffer_from_biguint(liquidity);

            self.direct_multi_esdt_transfer_execute(
                address,
                transfer,
                per_transfer_gas_limit,
                &endpoint,
                &arg,
            )?;
        }

        Ok(())
    }

    fn min_blocks_passed(&self, block: u64) -> bool {
        block > self.last_info_share_block().get() + self.info_share_min_blocks().get()
    }

    fn has_received_info(&self) -> bool {
        !self.received_info().is_empty()
    }

    fn arg_buffer_from_biguint(&self, biguint: &Self::BigUint) -> ArgBuffer {
        let mut args = ArgBuffer::new();
        args.push_argument_bytes(biguint.to_bytes_be().as_slice());
        args
    }

    fn compute_liquidity_amount_max_threshold(
        &self,
        liquidity_amount: &Self::BigUint,
    ) -> Self::BigUint {
        liquidity_amount * &11u64.into() / 10u64.into()
    }

    fn own_shared_info_set_if_empty_or_clear(&self, own_info: SharedInformation<Self::BigUint>) {
        if self.own_info().is_empty() {
            self.own_info().set(&own_info)
        } else {
            self.own_info().clear()
        }
    }

    fn own_shared_info_get_or_create(&self) -> SharedInformation<Self::BigUint> {
        if !self.own_info().is_empty() {
            self.own_info().get()
        } else {
            let res = self.new_own_shared_info();
            self.reset_safe_reserve();
            res
        }
    }

    fn new_own_shared_info(&self) -> SharedInformation<Self::BigUint> {
        SharedInformation {
            metadata: InformationMetadata {
                sender: self.blockchain().get_sc_address(),
                timestamp: self.blockchain().get_block_timestamp(),
            },
            liquidity_info: LiquidityInformation {
                liquidity_amount: self.safe_liquidity().get(),
                first_token_amount: self.safe_reserves_first().get(),
                second_token_amount: self.safe_reserves_second().get(),
            },
            swap_stats: SwapStatistics {
                _placeholder: Self::BigUint::zero(),
            },
        }
    }

    fn get_recv_info_decoded(&self) -> SCResult<Vec<SharedInformation<Self::BigUint>>> {
        let mut recv_info = Vec::new();
        for el in self.received_info().iter() {
            let decoded = SharedInformation::<Self::BigUint>::from_boxed_bytes(el.1)?;
            recv_info.push(decoded);
        }
        Ok(recv_info)
    }

    fn exteract_liquidity_info(
        &self,
        shared_info: &[SharedInformation<Self::BigUint>],
    ) -> Vec<LiquidityInformation<Self::BigUint>> {
        shared_info
            .iter()
            .map(|x| x.liquidity_info.clone())
            .collect()
    }

    fn compute_sum_all_liquidity(
        &self,
        all_liquidity_info: &[LiquidityInformation<Self::BigUint>],
    ) -> LiquidityInformation<Self::BigUint> {
        let mut liq_sum = LiquidityInformation {
            liquidity_amount: Self::BigUint::zero(),
            first_token_amount: Self::BigUint::zero(),
            second_token_amount: Self::BigUint::zero(),
        };

        all_liquidity_info.iter().for_each(|x| {
            liq_sum.liquidity_amount += &x.liquidity_amount;
            liq_sum.first_token_amount += &x.first_token_amount;
            liq_sum.second_token_amount += &x.second_token_amount;
        });

        liq_sum
    }

    #[view(getOwnInfo)]
    #[storage_mapper("Sharer:own_info")]
    fn own_info(&self) -> SingleValueMapper<Self::Storage, SharedInformation<Self::BigUint>>;

    #[view(getLastInfoShareEpoch)]
    #[storage_mapper("Sharer:last_info_share_block")]
    fn last_info_share_block(&self) -> SingleValueMapper<Self::Storage, u64>;
}
