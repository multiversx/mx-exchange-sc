multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use factory::locked_asset_token_merge::ProxyTrait as _;

pub type PaymentsVec<M> = ManagedVec<M, EsdtTokenPayment<M>>;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, Debug, PartialEq)]
pub struct UserEntry<M: ManagedTypeApi> {
    pub token_nonce: u64,
    pub stake_amount: BigUint<M>,
    pub unstake_amount: BigUint<M>,
    pub unbond_epoch: u64,
}

impl<M: ManagedTypeApi> UserEntry<M> {
    pub fn new(token_nonce: u64, stake_amount: BigUint<M>) -> Self {
        Self {
            token_nonce,
            stake_amount,
            unstake_amount: BigUint::zero(),
            unbond_epoch: u64::MAX,
        }
    }

    pub fn get_total_amount(&self) -> BigUint<M> {
        &self.stake_amount + &self.unstake_amount
    }
}

#[multiversx_sc::module]
pub trait LockedAssetTokenModule {
    fn require_all_locked_asset_payments(&self, payments: &PaymentsVec<Self::Api>) {
        require!(!payments.is_empty(), "No payments");

        let locked_asset_token_id = self.locked_asset_token_id().get();
        for p in payments {
            require!(
                p.token_identifier == locked_asset_token_id,
                "Invalid payment"
            );
        }
    }

    fn create_new_entry_by_merging_tokens(
        &self,
        entry_mapper: &SingleValueMapper<UserEntry<Self::Api>>,
        mut new_tokens: PaymentsVec<Self::Api>,
    ) -> UserEntry<Self::Api> {
        if entry_mapper.is_empty() {
            let merged_tokens = self.merge_locked_asset_tokens(new_tokens);

            return UserEntry::new(merged_tokens.token_nonce, merged_tokens.amount);
        }

        let mut prev_entry: UserEntry<Self::Api> = entry_mapper.get();
        let prev_entry_total_tokens = prev_entry.get_total_amount();
        self.total_locked_asset_supply()
            .update(|total_supply| *total_supply -= &prev_entry_total_tokens);

        let prev_entry_as_payment = EsdtTokenPayment::new(
            self.locked_asset_token_id().get(),
            prev_entry.token_nonce,
            prev_entry_total_tokens,
        );
        new_tokens.push(prev_entry_as_payment);

        let merged_tokens = self.merge_locked_asset_tokens(new_tokens);
        prev_entry.token_nonce = merged_tokens.token_nonce;
        prev_entry.stake_amount = &merged_tokens.amount - &prev_entry.unstake_amount;

        prev_entry
    }

    fn merge_locked_asset_tokens(
        &self,
        tokens: PaymentsVec<Self::Api>,
    ) -> EsdtTokenPayment<Self::Api> {
        if tokens.len() == 1 {
            return tokens.get(0);
        }

        let locked_asset_factory_address = self.locked_asset_factory_address().get();
        self.locked_asset_factory_proxy(locked_asset_factory_address)
            .merge_tokens()
            .with_multi_token_transfer(tokens)
            .execute_on_dest_context()
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

    #[storage_mapper("entryForUser")]
    fn entry_for_user(
        &self,
        user_address: &ManagedAddress,
    ) -> SingleValueMapper<UserEntry<Self::Api>>;

    #[view(getUserList)]
    #[storage_mapper("userList")]
    fn user_list(&self) -> UnorderedSetMapper<ManagedAddress>;
}
