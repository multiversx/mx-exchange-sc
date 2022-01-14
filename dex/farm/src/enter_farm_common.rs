elrond_wasm::imports!();

use crate::{custom_rewards, EnterFarmResultType};
use common_structs::FarmTokenAttributes;
use farm_token::FarmToken;

#[elrond_wasm::module]
pub trait EnterFarmCommon:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + crate::farm_token_merge::FarmTokenMergeModule
    + events::EventsModule
{
    fn enter_farm_common<MintRewardsFunc: Fn(&Self, &TokenIdentifier) -> BigUint>(
        &self,
        opt_accept_funds_func: OptionalArg<ManagedBuffer>,
        mint_rewards_function: MintRewardsFunc,
    ) -> SCResult<EnterFarmResultType<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let payments_vec = self.get_all_payments_managed_vec();
        let mut payments_iter = payments_vec.iter();
        let payment_0 = payments_iter.next().ok_or("empty payments")?;

        let token_in = payment_0.token_identifier.clone();
        let enter_amount = payment_0.amount.clone();

        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0, "Cannot farm with amount of 0");

        let farm_contribution = &enter_amount;
        let reward_token_id = self.reward_token_id().get();
        mint_rewards_function(self, &reward_token_id);

        let epoch = self.blockchain().get_block_epoch();
        let attributes = FarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            entering_epoch: epoch,
            original_entering_epoch: epoch,
            initial_farming_amount: enter_amount.clone(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: farm_contribution.clone(),
        };

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token_id().get();
        let (new_farm_token, created_with_merge) = self.create_farm_tokens_by_merging(
            farm_contribution,
            &farm_token_id,
            &attributes,
            payments_iter,
        )?;
        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &opt_accept_funds_func,
        )?;

        self.emit_enter_farm_event(
            &caller,
            &farming_token_id,
            &enter_amount,
            &new_farm_token.token_amount.token_identifier,
            new_farm_token.token_amount.token_nonce,
            &new_farm_token.token_amount.amount,
            &self.farm_token_supply().get(),
            &reward_token_id,
            &self.reward_reserve().get(),
            &new_farm_token.attributes,
            created_with_merge,
        );
        Ok(new_farm_token.token_amount)
    }

    fn create_farm_tokens_by_merging(
        &self,
        amount: &BigUint,
        token_id: &TokenIdentifier,
        attributes: &FarmTokenAttributes<Self::Api>,
        additional_payments: ManagedVecIterator<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<(FarmToken<Self::Api>, bool)> {
        let current_position_replic = FarmToken {
            token_amount: self.create_payment(token_id, 0, amount),
            attributes: attributes.clone(),
        };

        let additional_payments_len = additional_payments.len();
        let merged_attributes = self.get_merged_farm_token_attributes(
            additional_payments.clone(),
            Some(current_position_replic),
        )?;
        self.burn_farm_tokens_from_payments(additional_payments);

        let new_amount = &merged_attributes.current_farm_amount;
        let new_nonce = self.mint_farm_tokens(token_id, new_amount, &merged_attributes);

        let new_farm_token = FarmToken {
            token_amount: self.create_payment(token_id, new_nonce, new_amount),
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        Ok((new_farm_token, is_merged))
    }
}
