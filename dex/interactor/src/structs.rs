use multiversx_sc_scenario::{
    api::StaticApi,
    imports::{
        Address, BigInt, BigUint, EsdtTokenPayment, ManagedAddress, ManagedTypeApi, ManagedVec,
        MultiValue3, OptionalValue, RustBigUint, Sign, TokenIdentifier,
    },
    num_bigint,
};
use proxies::{ClaimDualYieldResult, Energy, StakeProxyResult, UnstakeResult};

use crate::{
    dex_interact_cli::{AddArgs, SwapArgs},
    DexInteract,
};

pub struct InteractorMultiValue3<T0, T1, T2>(pub (T0, T1, T2));

impl<T0, T1, T2> InteractorMultiValue3<T0, T1, T2> {
    pub fn new(first: T0, second: T1, third: T2) -> Self {
        InteractorMultiValue3((first, second, third))
    }
}
pub type InteractorAddLiquidityResultType =
    InteractorMultiValue3<InteractorPayment, InteractorPayment, InteractorPayment>;

impl<M: ManagedTypeApi>
    From<MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>>
    for InteractorAddLiquidityResultType
{
    fn from(
        value: MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>,
    ) -> Self {
        let extracted = value.0;

        InteractorMultiValue3::new(
            InteractorPayment::from(extracted.0),
            InteractorPayment::from(extracted.1),
            InteractorPayment::from(extracted.2),
        )
    }
}

pub type RustBigInt = num_bigint::BigInt;

#[allow(dead_code)]
pub struct InteractorUnstakeResult {
    pub other_token_payment: InteractorPayment,
    pub lp_farm_rewards: InteractorPayment,
    pub staking_rewards: InteractorPayment,
    pub unbond_staking_farm_token: InteractorPayment,
}

impl<M: ManagedTypeApi> From<UnstakeResult<M>> for InteractorUnstakeResult {
    fn from(value: UnstakeResult<M>) -> Self {
        InteractorUnstakeResult {
            other_token_payment: InteractorPayment::from(value.other_token_payment),
            lp_farm_rewards: InteractorPayment::from(value.lp_farm_rewards),
            staking_rewards: InteractorPayment::from(value.staking_rewards),
            unbond_staking_farm_token: InteractorPayment::from(value.unbond_staking_farm_token),
        }
    }
}

#[allow(dead_code)]
pub struct InteractorStakeProxyResult {
    pub dual_yield_tokens: InteractorPayment,
    pub staking_boosted_rewards: InteractorPayment,
    pub lp_farm_boosted_rewards: InteractorPayment,
}

impl<M: ManagedTypeApi> From<StakeProxyResult<M>> for InteractorStakeProxyResult {
    fn from(value: StakeProxyResult<M>) -> Self {
        InteractorStakeProxyResult {
            dual_yield_tokens: InteractorPayment::from(value.dual_yield_tokens),
            staking_boosted_rewards: InteractorPayment::from(value.staking_boosted_rewards),
            lp_farm_boosted_rewards: InteractorPayment::from(value.lp_farm_boosted_rewards),
        }
    }
}

#[allow(dead_code)]
pub struct InteractorClaimDualYieldResult {
    pub lp_farm_rewards: InteractorPayment,
    pub staking_farm_rewards: InteractorPayment,
    pub new_dual_yield_tokens: InteractorPayment,
}

impl<M: ManagedTypeApi> From<ClaimDualYieldResult<M>> for InteractorClaimDualYieldResult {
    fn from(value: ClaimDualYieldResult<M>) -> Self {
        InteractorClaimDualYieldResult {
            lp_farm_rewards: InteractorPayment::from(value.lp_farm_rewards),
            staking_farm_rewards: InteractorPayment::from(value.staking_farm_rewards),
            new_dual_yield_tokens: InteractorPayment::from(value.new_dual_yield_tokens),
        }
    }
}

pub struct InteractorFarmTokenAttributes {
    pub reward_per_share: RustBigUint,
    pub entering_epoch: u64,
    pub compounded_reward: RustBigUint,
    pub current_farm_amount: RustBigUint,
    pub original_owner: Address,
}

#[derive(Debug)]
pub struct InteractorPayment {
    pub token_id: String,
    pub nonce: u64,
    pub amount: RustBigUint,
}

#[allow(dead_code)]
pub struct InteractorEnergy {
    pub amount: RustBigInt,
    pub last_update_epoch: u64,
    pub total_locked_tokens: RustBigUint,
}

impl<M: ManagedTypeApi> From<Energy<M>> for InteractorEnergy {
    fn from(value: Energy<M>) -> Self {
        InteractorEnergy {
            amount: to_rust_bigint(value.amount),
            last_update_epoch: value.last_update_epoch,
            total_locked_tokens: to_rust_biguint(value.total_locked_tokens),
        }
    }
}

impl<M: ManagedTypeApi> From<EsdtTokenPayment<M>> for InteractorPayment {
    fn from(value: EsdtTokenPayment<M>) -> Self {
        InteractorPayment {
            token_id: value.token_identifier.to_string(),
            nonce: value.token_nonce,
            amount: to_rust_biguint(value.amount),
        }
    }
}

impl<M: ManagedTypeApi> From<InteractorPayment> for EsdtTokenPayment<M> {
    fn from(interactor_token: InteractorPayment) -> Self {
        EsdtTokenPayment::new(
            TokenIdentifier::from(interactor_token.token_id.as_bytes()),
            interactor_token.nonce,
            BigUint::from(interactor_token.amount),
        )
    }
}

impl<M: ManagedTypeApi> From<&InteractorPayment> for EsdtTokenPayment<M> {
    fn from(interactor_token: &InteractorPayment) -> Self {
        EsdtTokenPayment::new(
            TokenIdentifier::from(interactor_token.token_id.as_bytes()),
            interactor_token.nonce,
            BigUint::from(interactor_token.amount.clone()),
        )
    }
}

impl AddArgs {
    pub fn as_payment_vec(
        &self,
        dex_interact: &mut DexInteract,
    ) -> ManagedVec<StaticApi, EsdtTokenPayment<StaticApi>> {
        let first_token_id = dex_interact.state.first_token_id().as_bytes();
        let second_token_id = dex_interact.state.second_token_id().as_bytes();

        let mut payments = ManagedVec::from_single_item(EsdtTokenPayment::new(
            TokenIdentifier::from(first_token_id),
            0,
            BigUint::from(self.first_payment_amount),
        ));
        payments.push(EsdtTokenPayment::new(
            TokenIdentifier::from(second_token_id),
            0,
            BigUint::from(self.second_payment_amount),
        ));
        payments
    }
}

impl SwapArgs {
    pub fn as_payment(&self, dex_interact: &mut DexInteract) -> EsdtTokenPayment<StaticApi> {
        let first_token_id = dex_interact.state.first_token_id().as_bytes();
        EsdtTokenPayment::new(
            TokenIdentifier::from(first_token_id),
            0,
            BigUint::from(self.amount),
        )
    }
}

// helpers

pub fn extract_caller(
    dex_interact: &mut DexInteract,
    opt_original_caller: Option<Address>,
) -> OptionalValue<ManagedAddress<StaticApi>> {
    let caller = opt_original_caller.unwrap_or_else(|| dex_interact.wallet_address.to_address());
    OptionalValue::<ManagedAddress<StaticApi>>::Some(ManagedAddress::from(caller))
}

pub fn to_rust_biguint<M: ManagedTypeApi>(value: BigUint<M>) -> RustBigUint {
    RustBigUint::from_bytes_be(value.to_bytes_be().as_slice())
}

pub fn to_rust_bigint<M: ManagedTypeApi>(value: BigInt<M>) -> RustBigInt {
    let sign = value.sign();

    RustBigInt::from_bytes_be(to_rust_sign(sign), value.to_signed_bytes_be().as_slice())
}

pub fn to_rust_sign(value: Sign) -> num_bigint::Sign {
    match value {
        Sign::Minus => num_bigint::Sign::Minus,
        Sign::Plus => num_bigint::Sign::Plus,
        Sign::NoSign => num_bigint::Sign::NoSign,
    }
}
