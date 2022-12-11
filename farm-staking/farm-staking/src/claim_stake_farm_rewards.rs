elrond_wasm::imports!();

use farm::base_functions::ClaimRewardsResultType;

use crate::base_impl_wrapper::FarmStakingWrapper;

#[elrond_wasm::module]
pub trait ClaimStakeFarmRewardsModule:
    crate::custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + events::EventsModule
    + token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + sc_whitelist_module::SCWhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
        self.claim_rewards_common(None)
    }

    #[payable("*")]
    #[endpoint(claimRewardsWithNewValue)]
    fn claim_rewards_with_new_value(
        &self,
        new_farming_amount: BigUint,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        self.claim_rewards_common(Some(new_farming_amount))
    }

    fn claim_rewards_common(
        &self,
        opt_new_farming_amount: Option<BigUint>,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payment = self.call_value().single_esdt();
        let claim_result = self.claim_rewards_base_no_farm_token_mint::<FarmStakingWrapper<Self>>(
            caller.clone(),
            ManagedVec::from_single_item(payment),
        );

        let mut virtual_farm_token = claim_result.new_farm_token.clone();
        if let Some(new_amount) = opt_new_farming_amount {
            virtual_farm_token.payment.amount = new_amount.clone();
            virtual_farm_token.attributes.current_farm_amount = new_amount;
        }

        let new_farm_token_nonce = self.send().esdt_nft_create_compact(
            &virtual_farm_token.payment.token_identifier,
            &virtual_farm_token.payment.amount,
            &virtual_farm_token.attributes,
        );
        virtual_farm_token.payment.token_nonce = new_farm_token_nonce;

        self.send_payment_non_zero(&caller, &virtual_farm_token.payment);
        self.send_payment_non_zero(&caller, &claim_result.rewards);

        self.emit_claim_rewards_event(
            &caller,
            claim_result.context,
            virtual_farm_token.clone(),
            claim_result.rewards.clone(),
            claim_result.created_with_merge,
            claim_result.storage_cache,
        );

        (virtual_farm_token.payment, claim_result.rewards).into()
    }
}
