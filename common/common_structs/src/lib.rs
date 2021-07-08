#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub type Nonce = u64;
pub type Epoch = u64;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct FftTokenAmountPair<BigUint: BigUintApi> {
    pub token_id: TokenIdentifier,
    pub amount: BigUint,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi, Clone)]
pub struct GenericEsdtAmountPair<BigUint: BigUintApi> {
    pub token_id: TokenIdentifier,
    pub token_nonce: Nonce,
    pub amount: BigUint,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct TokenPair {
    pub first_token: TokenIdentifier,
    pub second_token: TokenIdentifier,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi, NestedEncode, NestedDecode, Clone, Copy)]
pub struct UnlockMilestone {
    pub unlock_epoch: u64,
    pub unlock_percent: u8,
}

#[derive(TopEncode, TopDecode, TypeAbi, Clone)]
pub struct WrappedLpTokenAttributes<BigUint: BigUintApi> {
    pub lp_token_id: TokenIdentifier,
    pub lp_token_total_amount: BigUint,
    pub locked_assets_invested: BigUint,
    pub locked_assets_nonce: Nonce,
}

#[derive(TopEncode, TopDecode, TypeAbi, Clone)]
pub struct WrappedFarmTokenAttributes<BigUint: BigUintApi> {
    pub farm_token_id: TokenIdentifier,
    pub farm_token_nonce: Nonce,
    pub farm_token_amount: BigUint,
    pub farming_token_id: TokenIdentifier,
    pub farming_token_nonce: Nonce,
    pub farming_token_amount: BigUint,
}

#[derive(TopEncode, TopDecode, TypeAbi, Clone)]
pub struct FarmTokenAttributes<BigUint: BigUintApi> {
    pub reward_per_share: BigUint,
    pub entering_epoch: u64,
    pub apr_multiplier: u8,
    pub with_locked_rewards: bool,
    pub initial_farming_amount: BigUint,
    pub compounded_reward: BigUint,
    pub current_farm_amount: BigUint,
}
