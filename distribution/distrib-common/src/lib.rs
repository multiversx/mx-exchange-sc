#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct UserAssetKey {
    pub user_address: Address,
    pub spread_epoch: u64,
    pub locked_asset: bool,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi, NestedEncode, NestedDecode, Clone, Copy)]
pub struct UnlockMilestone {
    pub unlock_epoch: u64,
    pub unlock_percent: u8,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct CommunityDistribution<BigUint: BigUintApi> {
    pub total_amount: BigUint,
    pub spread_epoch: u64,
    pub after_planning_amount: BigUint,
    pub unlock_milestones: Vec<UnlockMilestone>,
}

#[derive(TopEncode, TopDecode, Clone, TypeAbi)]
pub struct LockedTokenAttributes {
    pub unlock_milestones: Vec<UnlockMilestone>,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct WrappedLpTokenAttributes<BigUint: BigUintApi> {
    pub lp_token_id: TokenIdentifier,
    pub lp_token_total_amount: BigUint,
    pub locked_assets_invested: BigUint,
    pub locked_assets_nonce: Nonce,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct WrappedFarmTokenAttributes {
    pub farm_token_id: TokenIdentifier,
    pub farm_token_nonce: Nonce,
    pub farming_token_id: TokenIdentifier,
    pub farming_token_nonce: Nonce,
}
