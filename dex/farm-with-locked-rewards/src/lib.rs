#![no_std]
#![allow(clippy::too_many_arguments)]
#![feature(exact_size_is_empty)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use core::marker::PhantomData;

use common_structs::FarmTokenAttributes;
use contexts::storage_cache::StorageCache;

use farm::{
    base_functions::{BaseFunctionsModule, Wrapper},
    exit_penalty::{
        DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS, DEFAULT_PENALTY_PERCENT,
    },
};
use farm_base_impl::base_traits_impl::FarmContract;

type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[elrond_wasm::contract]
pub trait Farm:
    rewards::RewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + farm_token::FarmTokenModule
    + utils::UtilsModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + sc_whitelist_module::SCWhitelistModule
    + events::EventsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + farm::base_functions::BaseFunctionsModule
    + farm::exit_penalty::ExitPenaltyModule
    + farm_base_impl::base_farm_init::BaseFarmInitModule
    + farm_base_impl::base_farm_validation::BaseFarmValidationModule
    + farm_base_impl::enter_farm::BaseEnterFarmModule
    + farm_base_impl::claim_rewards::BaseClaimRewardsModule
    + farm_base_impl::compound_rewards::BaseCompoundRewardsModule
    + farm_base_impl::exit_farm::BaseExitFarmModule
    + farm_boosted_yields::FarmBoostedYieldsModule
    + week_timekeeping::WeekTimekeepingModule
    + weekly_rewards_splitting::WeeklyRewardsSplittingModule
    + weekly_rewards_splitting::ongoing_operation::OngoingOperationModule
    + weekly_rewards_splitting::events::WeeklyRewardsSplittingEventsModule
    + energy_query::EnergyQueryModule
{
    #[init]
    fn init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        pair_contract_address: ManagedAddress,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        self.base_farm_init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            owner,
            admins,
        );

        self.penalty_percent().set_if_empty(DEFAULT_PENALTY_PERCENT);
        self.minimum_farming_epochs()
            .set_if_empty(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set_if_empty(DEFAULT_BURN_GAS_LIMIT);
        self.pair_contract_address().set(&pair_contract_address);
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm_endpoint(
        &self,
        original_caller: ManagedAddress,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let output_farm_token_payment = self.enter_farm::<NoMintWrapper<Self>>(original_caller);
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards_endpoint(
        &self,
        original_caller: ManagedAddress,
    ) -> ClaimRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let payments = self.call_value().all_esdt_transfers();
        let base_claim_rewards_result =
            self.claim_rewards_base::<NoMintWrapper<Self>>(original_caller.clone(), payments);

        let output_farm_token_payment = base_claim_rewards_result.new_farm_token.payment.clone();
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        let rewards_payment = base_claim_rewards_result.rewards;
        let locked_rewards_payment = self.send_to_lock_contract_non_zero(
            rewards_payment.token_identifier,
            rewards_payment.amount,
            caller,
        );

        self.emit_claim_rewards_event::<_, FarmTokenAttributes<Self::Api>>(
            &original_caller,
            base_claim_rewards_result.context,
            base_claim_rewards_result.new_farm_token,
            locked_rewards_payment.clone(),
            base_claim_rewards_result.created_with_merge,
            base_claim_rewards_result.storage_cache,
        );

        (output_farm_token_payment, locked_rewards_payment).into()
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards_endpoint(
        &self,
        original_caller: ManagedAddress,
    ) -> CompoundRewardsResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let output_farm_token_payment =
            self.compound_rewards::<NoMintWrapper<Self>>(original_caller);
        self.send_payment_non_zero(&caller, &output_farm_token_payment);

        output_farm_token_payment
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm_endpoint(&self) -> ExitFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);

        let (farming_token_payment, reward_payment) = self
            .exit_farm::<NoMintWrapper<Self>>(caller.clone())
            .into_tuple();
        self.send_payment_non_zero(&caller, &farming_token_payment);
        let locked_rewards_payment = self.send_to_lock_contract_non_zero(
            reward_payment.token_identifier,
            reward_payment.amount,
            caller,
        );

        (farming_token_payment, locked_rewards_payment).into()
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        user: ManagedAddress,
        farm_token_amount: BigUint,
        attributes: FarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        self.require_queried();

        let mut storage_cache = StorageCache::new(self);
        NoMintWrapper::<Self>::generate_aggregated_rewards(self, &mut storage_cache);

        NoMintWrapper::<Self>::calculate_rewards(
            self,
            &user,
            &farm_token_amount,
            &attributes,
            &storage_cache,
        )
    }

    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens_endpoint(&self) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_sc_address_whitelisted(&caller);
        let new_tokens = self.merge_farm_tokens();
        self.send_payment_non_zero(&caller, &new_tokens);
        new_tokens
    }

    #[endpoint(startProduceRewards)]
    fn start_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.start_produce_rewards();
    }

    #[endpoint(endProduceRewards)]
    fn end_produce_rewards_endpoint(&self) {
        self.require_caller_has_admin_permissions();
        self.end_produce_rewards::<NoMintWrapper<Self>>();
    }

    #[endpoint(setPerBlockRewardAmount)]
    fn set_per_block_rewards_endpoint(&self, per_block_amount: BigUint) {
        self.require_caller_has_admin_permissions();
        self.set_per_block_rewards::<NoMintWrapper<Self>>(per_block_amount);
    }

    fn send_to_lock_contract_non_zero(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        destination_address: ManagedAddress,
    ) -> EsdtTokenPayment {
        if amount == 0 {
            return EsdtTokenPayment::new(token_id, 0, amount);
        }

        self.lock_virtual(token_id, amount, destination_address)
    }
}

pub struct NoMintWrapper<T: BaseFunctionsModule> {
    _phantom: PhantomData<T>,
}

impl<T> FarmContract for NoMintWrapper<T>
where
    T: BaseFunctionsModule,
{
    type FarmSc = T;
    type AttributesType = FarmTokenAttributes<<Self::FarmSc as ContractBase>::Api>;

    fn mint_rewards(
        _sc: &Self::FarmSc,
        _token_id: &TokenIdentifier<<Self::FarmSc as ContractBase>::Api>,
        _amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
    ) {
    }

    fn generate_aggregated_rewards(
        sc: &Self::FarmSc,
        storage_cache: &mut StorageCache<Self::FarmSc>,
    ) {
        Wrapper::<T>::generate_aggregated_rewards(sc, storage_cache);
    }

    fn calculate_rewards(
        sc: &Self::FarmSc,
        caller: &ManagedAddress<<Self::FarmSc as ContractBase>::Api>,
        farm_token_amount: &BigUint<<Self::FarmSc as ContractBase>::Api>,
        token_attributes: &Self::AttributesType,
        storage_cache: &StorageCache<Self::FarmSc>,
    ) -> BigUint<<Self::FarmSc as ContractBase>::Api> {
        Wrapper::<T>::calculate_rewards(
            sc,
            caller,
            farm_token_amount,
            token_attributes,
            storage_cache,
        )
    }
}
