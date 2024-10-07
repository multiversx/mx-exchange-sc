#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

pub type SnapshotEntry<M> = MultiValue2<ManagedAddress<M>, BigUint<M>>;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, Debug, PartialEq)]
pub struct UserEntry<M: ManagedTypeApi> {
    pub token_nonce: u64,
    pub stake_amount: BigUint<M>,
    pub unstake_amount: BigUint<M>,
    pub unbond_epoch: u64,
}

#[multiversx_sc::contract]
pub trait MetabondingStakingLegacy {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(stakeLockedAsset)]
    fn stake_locked_asset(&self) {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint]
    fn unstake(&self, _amount: BigUint) {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[endpoint]
    fn unbond(&self) {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getStakedAmountForUser)]
    fn get_staked_amount_for_user(&self, _user_address: ManagedAddress) -> BigUint {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getUserEntry)]
    fn get_user_entry(&self, _user_address: ManagedAddress) -> OptionalValue<UserEntry<Self::Api>> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    #[view(getSnapshot)]
    fn get_snapshot(&self) -> MultiValueEncoded<SnapshotEntry<Self::Api>> {
        sc_panic!("This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.");
    }

    // storage

    #[view(getLockedAssetTokenId)]
    #[storage_mapper("lockedAssetTokenId")]
    fn locked_asset_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLockedAssetFactoryAddress)]
    #[storage_mapper("lockedAssetFactoryAddress")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getTotalLockedAssetSupply)]
    #[storage_mapper("totalLockedAssetSupply")]
    fn total_locked_asset_supply(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("entryForUser")]
    fn entry_for_user(
        &self,
        user_address: &ManagedAddress,
    ) -> SingleValueMapper<UserEntry<Self::Api>>;

    #[view(getUserList)]
    #[storage_mapper("userList")]
    fn user_list(&self) -> UnorderedSetMapper<ManagedAddress>;
}
