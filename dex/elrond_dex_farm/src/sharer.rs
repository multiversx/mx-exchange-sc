elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::config;
use elrond_wasm::elrond_codec::TopEncode;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct SharedInformation<BigUint: BigUintApi> {
    pub metadata: InformationMetadata,
    pub farm_token_supply: BigUint,
    pub per_block_reward_amount: BigUint,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct InformationMetadata {
    pub sender: Address,
    pub timestamp: u64,
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
    + multitransfer::MultiTransferModule
    + config::ConfigModule
    + nft_deposit::NftDepositModule
    + token_send::TokenSendModule
    + token_supply::TokenSupplyModule
{
    #[endpoint(shareInformation)]
    fn share_information(&self) -> SCResult<()> {
        let block = self.blockchain().get_block_nonce();

        if block > self.last_info_share_block().get() + self.info_share_min_blocks().get() {
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

        let mut farm_token_supply_sum = own_info.farm_token_supply.clone();
        let mut total_rewards = own_info.per_block_reward_amount.clone();
        recv_info.iter().for_each(|x| {
            farm_token_supply_sum += &x.farm_token_supply;
            total_rewards += &x.per_block_reward_amount;
        });

        let new_per_block_reward_amount =
            &total_rewards * &own_info.farm_token_supply / farm_token_supply_sum;
        self.per_block_reward_amount()
            .set(&new_per_block_reward_amount);

        self.own_shared_info_set_if_empty_or_clear(own_info);
        Ok(())
    }

    fn arg_buffer_from_biguint(&self, biguint: &Self::BigUint) -> ArgBuffer {
        let mut args = ArgBuffer::new();
        args.push_argument_bytes(biguint.to_bytes_be().as_slice());
        args
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
            self.new_own_shared_info()
        }
    }

    fn new_own_shared_info(&self) -> SharedInformation<Self::BigUint> {
        SharedInformation {
            metadata: InformationMetadata {
                sender: self.blockchain().get_sc_address(),
                timestamp: self.blockchain().get_block_timestamp(),
            },
            farm_token_supply: self.get_farm_token_supply(),
            per_block_reward_amount: self.per_block_reward_amount().get(),
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

    #[view(getOwnInfo)]
    #[storage_mapper("Sharer:own_info")]
    fn own_info(&self) -> SingleValueMapper<Self::Storage, SharedInformation<Self::BigUint>>;

    #[view(getLastInfoShareEpoch)]
    #[storage_mapper("Sharer:last_info_share_block")]
    fn last_info_share_block(&self) -> SingleValueMapper<Self::Storage, u64>;
}
