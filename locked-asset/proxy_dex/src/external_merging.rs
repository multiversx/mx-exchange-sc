elrond_wasm::imports!();

use common_structs::PaymentsVec;
use elrond_wasm::api::CallTypeApi;

pub static FACTORY_MERGE_TOKENS_ENDPOINT_NAME: &[u8] = b"mergeTokens";
pub static FARM_MERGE_TOKENS_ENDPOINT_NAME: &[u8] = b"mergeFarmTokens";

pub fn merge_locked_tokens_through_factory<M: CallTypeApi>(
    original_caller: &ManagedAddress<M>,
    factory_address: ManagedAddress<M>,
    locked_tokens: PaymentsVec<M>,
) -> EsdtTokenPayment<M> {
    let merge_endpoint_name = ManagedBuffer::new_from_bytes(FACTORY_MERGE_TOKENS_ENDPOINT_NAME);
    let mut contract_call = ContractCall::<M, EsdtTokenPayment<M>>::new_with_esdt_payment(
        factory_address,
        merge_endpoint_name,
        locked_tokens,
    );
    contract_call.push_endpoint_arg(&original_caller);

    contract_call.execute_on_dest_context()
}

pub fn merge_farm_tokens_through_farm<M: CallTypeApi>(
    farm_address: ManagedAddress<M>,
    farm_tokens: PaymentsVec<M>,
) -> EsdtTokenPayment<M> {
    let merge_endpoint_name = ManagedBuffer::new_from_bytes(FARM_MERGE_TOKENS_ENDPOINT_NAME);
    let contract_call = ContractCall::<M, EsdtTokenPayment<M>>::new_with_esdt_payment(
        farm_address,
        merge_endpoint_name,
        farm_tokens,
    );

    contract_call.execute_on_dest_context()
}
