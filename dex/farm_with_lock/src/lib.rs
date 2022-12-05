#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

pub mod custom_rewards;

use common_errors_old::*;

use common_structs_old::FarmTokenAttributes;
use config::State;
use contexts::generic::GenericContext;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

mod factory {
    use common_structs_old::Epoch;

    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait FactoryProxy {
        #[endpoint(createAndForward)]
        fn create_and_forward(
            &self,
            amount: BigUint,
            address: ManagedAddress,
            start_epoch: Epoch,
            #[var_args] opt_accept_funds_fn: OptionalValue<ManagedBuffer>,
        ) -> EsdtTokenPayment<Self::Api>;
    }
}

#[elrond_wasm::contract]
pub trait Farm:
    custom_rewards::CustomRewardsModule
    + rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge_old::TokenMergeModule
    + farm_token::FarmTokenModule
    + events::EventsModule
    + contexts::ctx_helper::CtxHelper
    + migration_from_v1_2::MigrationModule
{
    #[proxy]
    fn locked_asset_factory(&self, to: ManagedAddress) -> factory::Proxy<Self::Api>;

    #[init]
    fn init(&self) {
        self.end_produce_rewards();
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(
        &self,
        #[var_args] opt_accept_funds_func: OptionalValue<ManagedBuffer>,
    ) -> ExitFarmResultType<Self::Api> {
        let mut context = self.new_farm_context(opt_accept_funds_func);

        self.load_state(&mut context);
        require!(
            context.get_contract_state().unwrap() == &State::Active,
            ERROR_NOT_ACTIVE
        );

        self.load_farm_token_id(&mut context);
        require!(
            !context.get_farm_token_id().unwrap().is_empty(),
            ERROR_NO_FARM_TOKEN
        );

        self.load_farming_token_id(&mut context);
        require!(context.is_accepted_payment_exit(), ERROR_BAD_PAYMENTS);

        self.load_reward_reserve(&mut context);
        self.load_reward_token_id(&mut context);
        self.load_block_nonce(&mut context);
        self.load_block_epoch(&mut context);
        self.load_reward_per_share(&mut context);
        self.load_farm_token_supply(&mut context);
        self.load_division_safety_constant(&mut context);
        self.load_farm_attributes(&mut context);

        self.generate_aggregated_rewards(context.get_storage_cache_mut());
        self.calculate_reward(&mut context);
        context.decrease_reward_reserve();
        self.calculate_initial_farming_amount(&mut context);
        self.increase_reward_with_compounded_rewards(&mut context);

        self.burn_position(&context);
        self.commit_changes(&context);

        self.send_rewards(&mut context);
        self.construct_output_payments_exit(&mut context);
        self.execute_output_payments(&context);
        self.emit_exit_farm_event(&context);

        self.construct_and_get_result(&context)
    }

    fn send_rewards(&self, context: &mut GenericContext<Self::Api>) {
        if context.get_position_reward().unwrap() > &0u64 {
            let locked_asset_factory_address = self.locked_asset_factory_address().get();
            let result = self
                .locked_asset_factory(locked_asset_factory_address)
                .create_and_forward(
                    context.get_position_reward().unwrap().clone(),
                    context.get_caller().clone(),
                    context.get_input_attributes().unwrap().entering_epoch,
                    context.get_opt_accept_funds_func().clone(),
                )
                .execute_on_dest_context_custom_range(|_, after| (after - 1, after));
            context.set_final_reward(result);
        } else {
            context.set_final_reward(self.create_payment(
                context.get_reward_token_id().unwrap(),
                0,
                context.get_position_reward().unwrap(),
            ));
        }
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        require!(amount > 0u64, ERROR_ZERO_AMOUNT);
        let farm_token_supply = self.farm_token_supply().get();
        require!(farm_token_supply >= amount, ERROR_ZERO_AMOUNT);

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let reward_increase =
            self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);
        let reward_per_share_increase = reward_increase * &self.division_safety_constant().get()
            / self.farm_token_supply().get();

        let future_reward_per_share = self.reward_per_share().get() + reward_per_share_increase;

        if future_reward_per_share > attributes.reward_per_share {
            let reward_per_share_diff = future_reward_per_share - attributes.reward_per_share;
            amount * &reward_per_share_diff / self.division_safety_constant().get()
        } else {
            BigUint::zero()
        }
    }

    fn burn_position(&self, context: &GenericContext<Self::Api>) {
        let farm_token = context.get_tx_input().get_payments().get_first();
        self.burn_farm_tokens(
            &farm_token.token_identifier,
            farm_token.token_nonce,
            &farm_token.amount,
        );
    }
}
