use common_structs::PaymentsVec;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, Copy)]
pub enum FarmHookType {
    // can't be done, execute_on_dest does not work on init
    _BeforeInitialize,
    _AfterInitialize,
    BeforeStake,
    AfterStake,
    BeforeClaimRewards,
    AfterClaimRewards,
    BeforeCompoundRewards,
    AfterCompoundRewards,
    BeforeUnstake,
    AfterUnstake,
    BeforeUnbond,
    AfterUnbond,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, PartialEq)]
pub struct Hook<M: ManagedTypeApi> {
    pub dest_address: ManagedAddress<M>,
    pub endpoint_name: ManagedBuffer<M>,
}

pub trait FarmHook {
    type Sc: ContractBase;

    fn before_stake(
        sc: &Self::Sc,
        farming_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        additional_farm_tokens: PaymentsVec<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_stake(
        sc: &Self::Sc,
        new_farm_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        boosted_rewards: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn before_claim_rewards(
        sc: &Self::Sc,
        farm_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_claim_rewards(
        sc: &Self::Sc,
        new_farm_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        rewards: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn before_compound_rewards(
        sc: &Self::Sc,
        farm_tokens: PaymentsVec<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_compound_rewards(
        sc: &Self::Sc,
        new_farm_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
        compounded_rewards: BigUint<<Self::Sc as ContractBase>::Api>,
    );

    fn before_unstake(
        sc: &Self::Sc,
        farm_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_unstake(
        sc: &Self::Sc,
        unbond_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        rewards: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn before_unbond(
        sc: &Self::Sc,
        unbond_token: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );

    fn after_unbond(
        sc: &Self::Sc,
        farming_tokens: EsdtTokenPayment<<Self::Sc as ContractBase>::Api>,
        original_caller: ManagedAddress<<Self::Sc as ContractBase>::Api>,
    );
}
