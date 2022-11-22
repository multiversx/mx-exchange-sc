elrond_wasm::imports!();

use common_structs::PaymentsVec;

use crate::base_impl_wrapper::FarmStakingWrapper;

#[elrond_wasm::module]
pub trait StakeFarmModule:
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
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(stakeFarmThroughProxy)]
    fn stake_farm_through_proxy(&self, staked_token_amount: BigUint) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let staked_token_id = self.farming_token_id().get();
        let staked_token_simulated_payment =
            EsdtTokenPayment::new(staked_token_id, 0, staked_token_amount);

        let farm_tokens = self.call_value().all_esdt_transfers();
        let mut payments = ManagedVec::from_single_item(staked_token_simulated_payment);
        payments.append_vec(farm_tokens);

        self.stake_farm_common(payments)
    }

    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(&self) -> EsdtTokenPayment {
        let payments = self.get_non_empty_payments();

        self.stake_farm_common(payments)
    }

    fn stake_farm_common(&self, payments: PaymentsVec<Self::Api>) -> EsdtTokenPayment {
        let caller = self.blockchain().get_caller();
        let enter_result =
            self.enter_farm_base::<FarmStakingWrapper<Self>>(caller.clone(), payments);

        let new_farm_token = enter_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &new_farm_token);

        self.emit_enter_farm_event(
            &caller,
            enter_result.context.farming_token_payment,
            enter_result.new_farm_token,
            enter_result.created_with_merge,
            enter_result.storage_cache,
        );

        new_farm_token
    }
}
