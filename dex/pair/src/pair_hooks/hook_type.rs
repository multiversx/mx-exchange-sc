use crate::pair_actions::swap::SwapType;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
pub enum PairHookType {
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

pub trait PairHook {
    type Sc: ContractBase;

    fn before_add_initial_liq(
        sc: &Self::Sc,
        first_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        second_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_add_initial_liq(
        sc: &Self::Sc,
        lp_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn before_add_liq(
        sc: &Self::Sc,
        first_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        second_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        first_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
        second_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
    );

    fn after_add_liq(
        sc: &Self::Sc,
        lp_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn before_remove_liq(
        sc: &Self::Sc,
        lp_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_remove_liq(
        sc: &Self::Sc,
        first_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        second_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        first_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
        second_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
    );

    fn before_swap(
        sc: &Self::Sc,
        payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        swap_type: SwapType,
    );

    fn after_swap(
        sc: &Self::Sc,
        payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        swap_type: SwapType,
        token_out: TokenIdentifier<<Self::Sc as ContractBase>::Api>,
        amount_out: BigUint<<Self::Sc as ContractBase>::Api>,
    );
}
