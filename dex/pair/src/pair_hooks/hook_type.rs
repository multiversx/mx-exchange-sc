multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
pub enum HookType {
    // can't be done, execute_on_dest does not work on init
    _BeforeInitialize,
    _AfterInitialize,
    BeforeAddInitialLiq,
    AfterAddInitialLiq,
    BeforeAddLiq,
    AfterAddLiq,
    BeforeRemoveLiq,
    AfterRemoveLiq,
    BeforeSwap,
    AfterSwap,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, PartialEq)]
pub struct Hook<M: ManagedTypeApi> {
    pub dest_address: ManagedAddress<M>,
    pub endpoint_name: ManagedBuffer<M>,
}
