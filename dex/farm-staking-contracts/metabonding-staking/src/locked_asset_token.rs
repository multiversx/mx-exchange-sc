elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use factory::locked_asset_token_merge::ProxyTrait as _;

pub type PaymentsVec<M> = ManagedVec<M, EsdtTokenPayment<M>>;

#[derive(TypeAbi, TopEncode, TopDecode, Debug, PartialEq)]
pub struct StakingEntry<M: ManagedTypeApi> {
    pub nonce: u64,
    pub amount: BigUint<M>,
    pub opt_unbond_epoch: Option<u64>,
}

impl<M: ManagedTypeApi> StakingEntry<M> {
    pub fn new(nonce: u64, amount: BigUint<M>) -> Self {
        Self {
            nonce,
            amount,
            opt_unbond_epoch: None,
        }
    }

    #[inline]
    pub fn is_unstaked(&self) -> bool {
        self.opt_unbond_epoch.is_some()
    }
}

#[elrond_wasm::module]
pub trait LockedAssetTokenModule {
    fn require_all_locked_asset_payments(&self, payments: &PaymentsVec<Self::Api>) {
        require!(payments.len() > 0, "No payments");

        let locked_asset_token_id = self.locked_asset_token_id().get();
        for p in payments {
            require!(
                p.token_identifier == locked_asset_token_id,
                "Invalid payment"
            );
        }
    }

    fn merge_locked_asset_tokens_if_needed(
        &self,
        user_address: &ManagedAddress,
        mut new_tokens: PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        let entry_mapper = self.staking_entry_for_user(user_address);
        let prev_entry_is_empty = entry_mapper.is_empty();
        if prev_entry_is_empty && new_tokens.len() == 1 {
            return new_tokens.get(0);
        }

        if !prev_entry_is_empty {
            let prev_entry = entry_mapper.get();
            require!(
                !prev_entry.is_unstaked(),
                "Cannot stake during unbond period"
            );

            self.total_locked_asset_supply()
                .update(|total_supply| *total_supply -= &prev_entry.amount);

            let prev_entry_as_payment = EsdtTokenPayment::new(
                self.locked_asset_token_id().get(),
                prev_entry.nonce,
                prev_entry.amount,
            );

            new_tokens.push(prev_entry_as_payment);
        }

        let locked_asset_factory_address = self.locked_asset_factory_address().get();
        self.locked_asset_factory_proxy(locked_asset_factory_address)
            .merge_locked_asset_tokens(OptionalValue::None)
            .with_multi_token_transfer(new_tokens)
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    // proxies

    #[proxy]
    fn locked_asset_factory_proxy(&self, sc_address: ManagedAddress) -> factory::Proxy<Self::Api>;

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

    #[storage_mapper("stakingEntryForUser")]
    fn staking_entry_for_user(
        &self,
        user_address: &ManagedAddress,
    ) -> SingleValueMapper<StakingEntry<Self::Api>>;

    #[view(getUserList)]
    #[storage_mapper("userList")]
    fn user_list(&self) -> UnorderedSetMapper<ManagedAddress>;
}
