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
    merge_common(
        original_caller,
        factory_address,
        merge_endpoint_name,
        locked_tokens,
    )
}

pub fn merge_farm_tokens_through_farm<M: CallTypeApi>(
    original_caller: &ManagedAddress<M>,
    farm_address: ManagedAddress<M>,
    farm_tokens: PaymentsVec<M>,
) -> EsdtTokenPayment<M> {
    let merge_endpoint_name = ManagedBuffer::new_from_bytes(FARM_MERGE_TOKENS_ENDPOINT_NAME);
    merge_common(
        original_caller,
        farm_address,
        merge_endpoint_name,
        farm_tokens,
    )
}

fn merge_common<M: CallTypeApi>(
    original_caller: &ManagedAddress<M>,
    sc_address: ManagedAddress<M>,
    endpoint_name: ManagedBuffer<M>,
    tokens: PaymentsVec<M>,
) -> EsdtTokenPayment<M> {
    let mut contract_call = ContractCall::<M, EsdtTokenPayment<M>>::new_with_esdt_payment(
        sc_address,
        endpoint_name,
        tokens,
    );
    contract_call.push_endpoint_arg(&original_caller);

    contract_call.execute_on_dest_context()
}
