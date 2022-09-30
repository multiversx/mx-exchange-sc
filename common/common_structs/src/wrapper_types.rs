elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{
    elrond_codec::TopEncode, CompoundedRewardAmountGetter, CurrentFarmAmountGetter,
    FarmTokenAttributes, InitialFarmingAmountGetter, PaymentAmountGetter, RewardPerShareGetter,
};

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Eq)]
pub struct TokenPair<M: ManagedTypeApi> {
    pub first_token: TokenIdentifier<M>,
    pub second_token: TokenIdentifier<M>,
}

impl<M: ManagedTypeApi> TokenPair<M> {
    pub fn equals(&self, other: &TokenPair<M>) -> bool {
        self.first_token == other.first_token && self.second_token == other.second_token
    }
}

#[derive(
    TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, Clone, Debug,
)]
pub struct EpochAmountPair<M: ManagedTypeApi> {
    pub epoch: u64,
    pub amount: BigUint<M>,
}

#[derive(Clone)]
pub struct PaymentAttributesPair<
    M: ManagedTypeApi,
    T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode,
> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: T,
}

pub type DefaultFarmPaymentAttributesPair<M> = PaymentAttributesPair<M, FarmTokenAttributes<M>>;

impl<M: ManagedTypeApi, T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode>
    PaymentAmountGetter<M> for PaymentAttributesPair<M, T>
{
    fn get_payment_amount(&self) -> &BigUint<M> {
        &self.payment.amount
    }
}

impl<
        M: ManagedTypeApi,
        T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + RewardPerShareGetter<M>,
    > RewardPerShareGetter<M> for PaymentAttributesPair<M, T>
{
    fn get_reward_per_share(&self) -> &BigUint<M> {
        self.attributes.get_reward_per_share()
    }
}

impl<
        M: ManagedTypeApi,
        T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + InitialFarmingAmountGetter<M>,
    > InitialFarmingAmountGetter<M> for PaymentAttributesPair<M, T>
{
    fn get_initial_farming_amount(&self) -> &BigUint<M> {
        self.attributes.get_initial_farming_amount()
    }
}

impl<
        M: ManagedTypeApi,
        T: Clone + TopEncode + TopDecode + NestedEncode + NestedDecode + CurrentFarmAmountGetter<M>,
    > CurrentFarmAmountGetter<M> for PaymentAttributesPair<M, T>
{
    fn get_current_farm_amount(&self) -> &BigUint<M> {
        self.attributes.get_current_farm_amount()
    }
}

impl<
        M: ManagedTypeApi,
        T: Clone
            + TopEncode
            + TopDecode
            + NestedEncode
            + NestedDecode
            + CompoundedRewardAmountGetter<M>,
    > CompoundedRewardAmountGetter<M> for PaymentAttributesPair<M, T>
{
    fn get_compounded_reward_amount(&self) -> &BigUint<M> {
        self.attributes.get_compounded_reward_amount()
    }
}
