#![no_std]

use core::marker::PhantomData;

use pair::pair_hooks::hook_type::PairHook;

multiversx_sc::imports!();

#[multiversx_sc::contract]
pub trait PairHooksSample {
    #[init]
    fn init(&self, known_pairs: MultiValueEncoded<ManagedAddress>) {
        let mapper = self.known_pairs();
        for pair in known_pairs {
            mapper.add(&pair);
        }
    }

    #[payable("*")]
    #[endpoint(beforeAddInitialLiqHook)]
    fn before_add_initial_liq_hook(&self, original_caller: ManagedAddress) {
        self.require_known_pair();

        let [first_payment, second_payment] = self.call_value().multi_esdt();
        Wrapper::<Self>::before_add_initial_liq(
            self,
            first_payment,
            second_payment,
            original_caller,
        );
    }

    #[payable("*")]
    #[endpoint(afterAddInitialLiqHook)]
    fn after_add_initial_liq_hook(&self, original_caller: ManagedAddress) {
        self.require_known_pair();

        let lp_payment = self.call_value().single_esdt();
        Wrapper::<Self>::after_add_initial_liq(self, lp_payment, original_caller);
    }

    fn require_known_pair(&self) {
        let caller = self.blockchain().get_caller();
        require!(
            self.known_pairs().contains(&caller),
            "Only known pairs may call this endpoint"
        );
    }

    #[storage_mapper("knownPairs")]
    fn known_pairs(&self) -> WhitelistMapper<ManagedAddress>;
}

pub struct Wrapper<T: PairHooksSample> {
    _phantom: PhantomData<T>,
}

impl<T: PairHooksSample> PairHook for Wrapper<T> {
    type Sc = T;

    fn before_add_initial_liq(
        sc: &Self::Sc,
        first_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        second_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    ) {
        let caller = sc.blockchain().get_caller();
        sc.send()
            .direct_non_zero_esdt_payment(&caller, &first_payment);
        sc.send()
            .direct_non_zero_esdt_payment(&caller, &second_payment);
    }

    fn after_add_initial_liq(
        sc: &Self::Sc,
        lp_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    ) {
        let caller = sc.blockchain().get_caller();
        sc.send().direct_non_zero_esdt_payment(&caller, &lp_payment);
    }

    fn before_add_liq(
        _sc: &Self::Sc,
        _first_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _second_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        _first_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
        _second_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
    ) {
        todo!()
    }

    fn after_add_liq(
        _sc: &Self::Sc,
        _lp_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    ) {
        todo!()
    }

    fn before_remove_liq(
        _sc: &Self::Sc,
        _lp_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    ) {
        todo!()
    }

    fn after_remove_liq(
        _sc: &Self::Sc,
        _first_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _second_payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        _first_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
        _second_token_amount_min: BigUint<<Self::Sc as ContractBase>::Api>,
    ) {
        todo!()
    }

    fn before_swap(
        _sc: &Self::Sc,
        _payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        _swap_type: pair::pair_actions::swap::SwapType,
    ) {
        todo!()
    }

    fn after_swap(
        _sc: &Self::Sc,
        _payment: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        _original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        _swap_type: pair::pair_actions::swap::SwapType,
        _token_out: TokenIdentifier<<Self::Sc as ContractBase>::Api>,
        _amount_out: BigUint<<Self::Sc as ContractBase>::Api>,
    ) {
        todo!()
    }
}
