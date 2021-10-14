elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Epoch, LockedAssetTokenAttributes, Nonce, UnlockMilestone};

pub const PERCENTAGE_TOTAL: u64 = 100;

#[elrond_wasm::module]
pub trait LockedAssetModule: token_supply::TokenSupplyModule + token_send::TokenSendModule {
    fn create_and_send_locked_assets(
        &self,
        amount: &Self::BigUint,
        additional_amount_to_create: &Self::BigUint,
        address: &Address,
        attributes: &LockedAssetTokenAttributes,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<Nonce> {
        let token_id = self.locked_asset_token_id().get();
        self.create_tokens(
            &token_id,
            &(amount + additional_amount_to_create),
            attributes,
        );
        let last_created_nonce = self.locked_asset_token_nonce().get();
        self.send_nft_tokens(
            &token_id,
            last_created_nonce,
            amount,
            address,
            opt_accept_funds_func,
        )?;
        Ok(last_created_nonce)
    }

    fn add_quantity_and_send_locked_assets(
        &self,
        amount: &Self::BigUint,
        sft_nonce: Nonce,
        address: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let token_id = self.locked_asset_token_id().get();
        self.nft_add_quantity_tokens(&token_id, sft_nonce, amount);
        self.send_nft_tokens(&token_id, sft_nonce, amount, address, opt_accept_funds_func)
    }

    fn create_tokens(
        &self,
        token: &TokenIdentifier,
        amount: &Self::BigUint,
        attributes: &LockedAssetTokenAttributes,
    ) {
        self.nft_create_tokens(token, amount, attributes);
        self.increase_nonce();
    }

    fn get_unlock_amount(
        &self,
        amount: &Self::BigUint,
        current_epoch: Epoch,
        unlock_milestones: &[UnlockMilestone],
    ) -> Self::BigUint {
        amount * &(self.get_unlock_percent(current_epoch, unlock_milestones) as u64).into()
            / PERCENTAGE_TOTAL.into()
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

        if unlock_percent > 100 {
            self.send().signal_error(b"unlock percent greater than 100");
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
        let unlock_percent_remaining = PERCENTAGE_TOTAL - (unlock_percent as u64);

        if unlock_percent_remaining == 0 {
            return new_unlock_milestones;
        }

        for old_milestone in old_unlock_milestones.iter() {
            if old_milestone.unlock_epoch > current_epoch {
                let new_unlock_percent: u64 = (old_milestone.unlock_percent as u64)
                    * PERCENTAGE_TOTAL
                    / unlock_percent_remaining;
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
        new_unlock_milestones[0].unlock_percent += PERCENTAGE_TOTAL as u8 - sum_of_new_percents;
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
        require!(!unlock_milestones.is_empty(), "Empty param");

        let mut percents_sum: u8 = 0;
        let mut last_milestone_unlock_epoch: u64 = 0;

        for milestone in unlock_milestones.0.clone() {
            require!(
                milestone.unlock_epoch >= last_milestone_unlock_epoch,
                "Unlock epochs not in order"
            );
            require!(
                milestone.unlock_percent <= 100,
                "Unlock percent more than 100"
            );
            last_milestone_unlock_epoch = milestone.unlock_epoch;
            percents_sum += milestone.unlock_percent;
        }

        require!(percents_sum == 100, "Percents do not sum up to 100");
        Ok(())
    }

    fn get_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<LockedAssetTokenAttributes> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        let farm_attributes = token_info.decode_attributes::<LockedAssetTokenAttributes>();
        match farm_attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    #[endpoint]
    fn set_transfer_exec_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        self.transfer_exec_gas_limit().set(&gas_limit);
        Ok(())
    }

    fn mint_and_send_assets(&self, dest: &Address, amount: &Self::BigUint) {
        if amount > &0 {
            let asset_token_id = self.asset_token_id().get();
            self.mint_tokens(&asset_token_id, amount);
            self.send().direct(dest, &asset_token_id, 0, amount, &[]);
        }
    }

    #[view(getLockedAssetTokenId)]
    #[storage_mapper("locked_asset_token_id")]
    fn locked_asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("locked_token_nonce")]
    fn locked_asset_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[view(getAssetTokenId)]
    #[storage_mapper("asset_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
