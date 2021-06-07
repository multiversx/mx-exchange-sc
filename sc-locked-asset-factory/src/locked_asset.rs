elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;
type Epoch = u64;

use distrib_common::*;
use modules::*;

const ADDITIONAL_AMOUNT_TO_CREATE: u64 = 1;

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, TypeAbi)]
pub struct UnlockSchedule {
    pub unlock_milestones: Vec<UnlockMilestone>,
}

#[elrond_wasm_derive::module]
pub trait LockedAssetModule: asset::AssetModule {
    fn create_and_send_locked_assets(
        &self,
        amount: &Self::BigUint,
        address: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> Nonce {
        let token_id = self.locked_asset_token_id().get();
        self.create_tokens(&token_id, amount);
        let last_created_nonce = self.locked_asset_token_nonce().get();
        self.send_tokens(
            &token_id,
            last_created_nonce,
            amount,
            address,
            opt_accept_funds_func,
        );
        last_created_nonce
    }

    fn add_quantity_and_send_locked_assets(
        &self,
        amount: &Self::BigUint,
        sft_nonce: Nonce,
        address: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) {
        let token_id = self.locked_asset_token_id().get();
        self.send()
            .esdt_nft_add_quantity(&token_id, sft_nonce, amount);
        self.send_tokens(&token_id, sft_nonce, amount, address, opt_accept_funds_func);
    }

    fn send_tokens(
        &self,
        token: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
        destination: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) {
        let (function, gas_limit) = match opt_accept_funds_func {
            OptionalArg::Some(accept_funds_func) => (
                accept_funds_func.as_slice(),
                self.transfer_exec_gas_limit().get(),
            ),
            OptionalArg::None => {
                let no_func: &[u8] = &[];
                (no_func, 0u64)
            }
        };

        let _ = self.send().direct_esdt_nft_execute(
            destination,
            token,
            nonce,
            amount,
            gas_limit,
            function,
            &ArgBuffer::new(),
        );
    }

    fn create_tokens(&self, token: &TokenIdentifier, amount: &Self::BigUint) {
        let amount_to_create = amount + &Self::BigUint::from(ADDITIONAL_AMOUNT_TO_CREATE);
        self.send().esdt_nft_create(
            token,
            &amount_to_create,
            &BoxedBytes::empty(),
            &Self::BigUint::zero(),
            &BoxedBytes::empty(),
            &BoxedBytes::empty(),
            &[BoxedBytes::empty()],
        );
        self.increase_nonce();
    }

    fn get_unlock_amount(
        &self,
        amount: &Self::BigUint,
        current_epoch: Epoch,
        unlock_milestones: &[UnlockMilestone],
    ) -> Self::BigUint {
        amount
            * &Self::BigUint::from(self.get_unlock_percent(current_epoch, unlock_milestones) as u64)
            / Self::BigUint::from(100u64)
    }

    fn get_unlock_percent(
        &self,
        current_epoch: Epoch,
        unlock_milestones: &[UnlockMilestone],
    ) -> u8 {
        let mut unlock_percent = 0u8;

        for milestone in unlock_milestones {
            if milestone.unlock_epoch <= current_epoch {
                unlock_percent += milestone.unlock_percent;
            }
        }
        unlock_percent
    }

    fn create_new_unlock_milestones(
        &self,
        current_epoch: Epoch,
        old_unlock_milestones: &[UnlockMilestone],
    ) -> Vec<UnlockMilestone> {
        let mut new_unlock_milestones = Vec::<UnlockMilestone>::new();
        let unlock_percent = self.get_unlock_percent(current_epoch, old_unlock_milestones);
        let unlock_percent_remaining = 100u64 - (unlock_percent as u64);

        if unlock_percent_remaining == 0 {
            return new_unlock_milestones;
        }

        for old_milestone in old_unlock_milestones.iter() {
            if old_milestone.unlock_epoch > current_epoch {
                let new_unlock_percent: u64 =
                    (old_milestone.unlock_percent as u64) * 100u64 / unlock_percent_remaining;
                new_unlock_milestones.push(UnlockMilestone {
                    unlock_epoch: old_milestone.unlock_epoch,
                    unlock_percent: new_unlock_percent as u8,
                });
            }
        }
        let mut sum_of_new_percents = 0u8;

        for new_milestone in new_unlock_milestones.iter() {
            sum_of_new_percents += new_milestone.unlock_percent;
        }
        new_unlock_milestones[0].unlock_percent += 100 - sum_of_new_percents;
        new_unlock_milestones
    }

    fn increase_nonce(&self) -> Nonce {
        let new_nonce = self.locked_asset_token_nonce().get() + 1;
        self.locked_asset_token_nonce().set(&new_nonce);
        new_nonce
    }

    fn validate_unlock_milestones(
        &self,
        unlock_milestones: &VarArgs<UnlockMilestone>,
    ) -> SCResult<()> {
        let mut percents_sum: u8 = 0;
        let mut last_milestone_unlock_epoch: u64 = 0;

        for milestone in unlock_milestones.0.clone() {
            require!(
                milestone.unlock_epoch > last_milestone_unlock_epoch,
                "Unlock epochs not in order"
            );
            require!(
                milestone.unlock_percent <= 100,
                "Unlock percent more than 100"
            );
            last_milestone_unlock_epoch = milestone.unlock_epoch;
            percents_sum += milestone.unlock_percent;
        }

        if !unlock_milestones.is_empty() {
            require!(percents_sum == 100, "Percents do not sum up to 100");
        }
        Ok(())
    }

    #[endpoint]
    fn set_transfer_exec_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.transfer_exec_gas_limit().set(&gas_limit);
        Ok(())
    }

    #[storage_mapper("transfer_exec_gas_limit")]
    fn transfer_exec_gas_limit(&self) -> SingleValueMapper<Self::Storage, u64>;

    #[view(getLockedAssetTokenId)]
    #[storage_mapper("locked_asset_token_id")]
    fn locked_asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("locked_token_nonce")]
    fn locked_asset_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;
}
