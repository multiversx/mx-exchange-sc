elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::FarmTokenAttributes;
use elrond_wasm::api::{CallTypeApi, StorageMapperApi};
use farm_token::FarmToken;

use crate::storage_cache::FarmContracTraitBounds;

pub struct GenericContext<M: ManagedTypeApi> {
    caller: ManagedAddress<M>,
    tx_input: GenericTxInput<M>,
    block_nonce: u64,
    block_epoch: u64,
    position_reward: BigUint<M>,
    initial_farming_amount: BigUint<M>,
    final_reward: Option<EsdtTokenPayment<M>>,
    output_attributes: Option<FarmTokenAttributes<M>>,
    output_created_with_merge: bool,
    output_payments: ManagedVec<M, EsdtTokenPayment<M>>,
}

pub struct GenericTxInput<M: ManagedTypeApi> {
    pub first_payment: EsdtTokenPayment<M>,
    pub additional_payments: ManagedVec<M, EsdtTokenPayment<M>>,
    attributes: Option<FarmTokenAttributes<M>>,
}

impl<M: ManagedTypeApi + StorageMapperApi + CallTypeApi + CallValueApi> GenericTxInput<M> {
    pub fn new<C: FarmContracTraitBounds<Api = M>>(farm_sc: &C) -> Self {
        let mut payments = farm_sc.call_value().all_esdt_transfers();

        let first_payment = payments.get(0);
        payments.remove(0);

        GenericTxInput {
            first_payment,
            additional_payments: payments,
            attributes: None,
        }
    }
}

impl<M: ManagedTypeApi + BlockchainApi + StorageMapperApi + CallTypeApi + CallValueApi>
    GenericContext<M>
{
    /*
    pub fn set_output_position(&mut self, position: FarmToken<M>, created_with_merge: bool) {
        self.output_payments.push(position.payment);
        self.output_created_with_merge = created_with_merge;
        self.output_attributes = Some(position.attributes);
    }

    pub fn set_final_reward_for_emit_compound_event(&mut self) {
        self.final_reward = Some(EsdtTokenPayment::new(
            self.storage_cache.reward_token_id.clone(),
            0,
            self.position_reward.clone(),
        ));
    }
    */
}
