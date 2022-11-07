elrond_wasm::imports!();

use farm::base_functions::ExitFarmResultType;

use crate::{base_impl_wrapper::FarmStakingWrapper, token_attributes::UnbondSftAttributes};

#[elrond_wasm::module]
pub trait UnstakeFarmModule:
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
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(&self) -> ExitFarmResultType<Self::Api> {
        let payment = self.call_value().single_esdt();

        self.unstake_farm_common(payment, None)
    }

    #[payable("*")]
    #[endpoint(unstakeFarmThroughProxy)]
    fn unstake_farm_through_proxy(&self) -> ExitFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let [first_payment, second_payment] = self.call_value().multi_esdt();

        // first payment are the staking tokens, taken from the liquidity pool
        // they will be sent to the user on unbond
        let staking_token_id = self.farming_token_id().get();
        require!(
            first_payment.token_identifier == staking_token_id,
            "Invalid staking token received"
        );

        self.unstake_farm_common(second_payment, Some(first_payment.amount))
    }

    fn unstake_farm_common(
        &self,
        payment: EsdtTokenPayment,
        opt_unbond_amount: Option<BigUint>,
    ) -> ExitFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        let exit_result = self.exit_farm_base::<FarmStakingWrapper<Self>>(caller.clone(), payment);

        let unbond_token_amount =
            opt_unbond_amount.unwrap_or(exit_result.farming_token_payment.amount);
        let farm_token_id = exit_result.storage_cache.farm_token_id.clone();
        let unbond_farm_token =
            self.create_and_send_unbond_tokens(&caller, farm_token_id, unbond_token_amount);

        self.send_payment_non_zero(&caller, &exit_result.reward_payment);

        self.emit_exit_farm_event(
            &caller,
            exit_result.context,
            unbond_farm_token.clone(),
            exit_result.reward_payment.clone(),
            exit_result.storage_cache,
        );

        (unbond_farm_token, exit_result.reward_payment).into()
    }

    fn create_and_send_unbond_tokens(
        &self,
        to: &ManagedAddress,
        farm_token_id: TokenIdentifier,
        amount: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
        let min_unbond_epochs = self.min_unbond_epochs().get();
        let current_epoch = self.blockchain().get_block_epoch();
        let nft_nonce = self.send().esdt_nft_create_compact(
            &farm_token_id,
            &amount,
            &UnbondSftAttributes {
                unlock_epoch: current_epoch + min_unbond_epochs,
            },
        );
        self.send()
            .direct_esdt(to, &farm_token_id, nft_nonce, &amount);

        EsdtTokenPayment::new(farm_token_id, nft_nonce, amount)
    }
}
