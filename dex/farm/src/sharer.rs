elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::config;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct SharedInformation<M: ManagedTypeApi> {
    pub metadata: InformationMetadata<M>,
    pub farm_token_supply: BigUint<M>,
    pub per_block_reward_amount: BigUint<M>,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, PartialEq, Clone)]
pub struct InformationMetadata<M: ManagedTypeApi> {
    pub sender: ManagedAddress<M>,
    pub timestamp: u64,
}

#[elrond_wasm::module]
pub trait SharerModule:
    info_sync::InfoSyncModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_supply::TokenSupplyModule
{
    fn info_to_managed_buffer(&self, info: &SharedInformation<Self::Api>) -> ManagedBuffer {
        self.serializer().top_encode_to_managed_buffer(info)
    }

    fn managed_buffer_to_info(&self, buff: &ManagedBuffer) -> SharedInformation<Self::Api> {
        self.serializer().top_decode_from_managed_buffer(buff)
    }

    #[endpoint(shareInformation)]
    fn share_information(&self) -> SCResult<()> {
        let block = self.blockchain().get_block_nonce();

        if block > self.last_info_share_block().get() + self.info_share_min_blocks().get() {
            self.last_info_share_block().set(&block);
            let shared_info = self.own_shared_info_get_or_create();
            let shared_info_buff = self.info_to_managed_buffer(&shared_info);
            require!(!shared_info_buff.is_empty(), "Error encoding");
            self.broadcast_information(shared_info_buff)?;
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

    fn arg_buffer_from_biguint(&self, biguint: &BigUint) -> ArgBuffer {
        let mut args = ArgBuffer::new();
        args.push_argument_bytes(biguint.to_bytes_be().as_slice());
        args
    }

    fn own_shared_info_set_if_empty_or_clear(&self, own_info: SharedInformation<Self::Api>) {
        if self.own_info().is_empty() {
            self.own_info().set(&own_info)
        } else {
            self.own_info().clear()
        }
    }

    fn own_shared_info_get_or_create(&self) -> SharedInformation<Self::Api> {
        if !self.own_info().is_empty() {
            self.own_info().get()
        } else {
            self.new_own_shared_info()
        }
    }

    fn new_own_shared_info(&self) -> SharedInformation<Self::Api> {
        SharedInformation {
            metadata: InformationMetadata {
                sender: self.blockchain().get_sc_address(),
                timestamp: self.blockchain().get_block_timestamp(),
            },
            farm_token_supply: self.get_farm_token_supply(),
            per_block_reward_amount: self.per_block_reward_amount().get(),
        }
    }

    fn get_recv_info_decoded(&self) -> SCResult<Vec<SharedInformation<Self::Api>>> {
        let mut recv_info = Vec::new();
        for el in self.received_info().iter() {
            let decoded = self.managed_buffer_to_info(&el.1);
            recv_info.push(decoded);
        }
        Ok(recv_info)
    }

    #[view(getOwnInfo)]
    #[storage_mapper("Sharer:own_info")]
    fn own_info(&self) -> SingleValueMapper<SharedInformation<Self::Api>>;

    #[view(getLastInfoShareEpoch)]
    #[storage_mapper("Sharer:last_info_share_block")]
    fn last_info_share_block(&self) -> SingleValueMapper<u64>;
}
