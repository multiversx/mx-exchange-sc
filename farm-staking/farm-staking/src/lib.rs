#![no_std]
#![feature(generic_associated_types)]
#![feature(exact_size_is_empty)]
#![allow(clippy::too_many_arguments)]

pub mod custom_rewards;
pub mod farm_token_merge;
pub mod whitelist;

use common_structs::Nonce;
use pausable::State;
use permissions_module::Permissions;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::farm_token_merge::StakingFarmTokenAttributes;
use config::{DEFAULT_BURN_GAS_LIMIT, DEFAULT_MINUMUM_FARMING_EPOCHS};
use farm_token_merge::StakingFarmToken;

pub type EnterFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type CompoundRewardsResultType<BigUint> = EsdtTokenPayment<BigUint>;
pub type ClaimRewardsResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
pub type ExitFarmResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;
pub type UnbondFarmResultType<BigUint> = EsdtTokenPayment<BigUint>;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct UnbondSftAttributes {
    pub unlock_epoch: u64,
}

#[elrond_wasm::contract]
pub trait Farm:
    custom_rewards::CustomRewardsModule
    + config::ConfigModule
    + token_send::TokenSendModule
    + token_merge::TokenMergeModule
    + farm_token::FarmTokenModule
    + farm_token_merge::FarmTokenMergeModule
    + whitelist::WhitelistModule
    + pausable::PausableModule
    + permissions_module::PermissionsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        max_apr: BigUint,
        min_unbond_epochs: u64,
        admins: MultiValueEncoded<ManagedAddress>,
    ) {
        require!(
            farming_token_id.is_valid_esdt_identifier(),
            "Farming token ID is not a valid esdt identifier"
        );
        require!(
            division_safety_constant != 0,
            "Division constant cannot be 0"
        );

        let farm_token = self.farm_token().get_token_id();
        require!(
            farming_token_id != farm_token,
            "Farming token ID cannot be farm token ID"
        );
        require!(max_apr > 0u64, "Invalid max APR percentage");

        self.state().set(&State::Inactive);
        self.minimum_farming_epochs()
            .set_if_empty(DEFAULT_MINUMUM_FARMING_EPOCHS);
        self.burn_gas_limit().set_if_empty(DEFAULT_BURN_GAS_LIMIT);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        // farming and reward token are the same
        self.reward_token_id().set(&farming_token_id);
        self.farming_token_id().set(&farming_token_id);
        self.max_annual_percentage_rewards().set(&max_apr);
        self.try_set_min_unbond_epochs(min_unbond_epochs);

        let caller = self.blockchain().get_caller();
        if admins.is_empty() {
            // backwards compatibility
            let all_permissions = Permissions::OWNER | Permissions::ADMIN | Permissions::PAUSE;
            self.set_permissions(caller, all_permissions);
        } else {
            self.set_permissions(caller, Permissions::OWNER | Permissions::PAUSE);
            self.set_permissions_for_all(admins, Permissions::ADMIN);
        };
    }

    #[payable("*")]
    #[endpoint(stakeFarmThroughProxy)]
    fn stake_farm_through_proxy(
        &self,
        staked_token_amount: BigUint,
    ) -> EnterFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller);

        let staked_token_id = self.farming_token_id().get();
        let staked_token_simulated_payment =
            EsdtTokenPayment::new(staked_token_id, 0, staked_token_amount);

        let farm_tokens = self.call_value().all_esdt_transfers();
        let mut payments = ManagedVec::from_single_item(staked_token_simulated_payment);
        payments.append_vec(farm_tokens);

        self.stake_farm(payments)
    }

    #[payable("*")]
    #[endpoint(stakeFarm)]
    fn stake_farm_endpoint(&self) -> EnterFarmResultType<Self::Api> {
        let payments = self.call_value().all_esdt_transfers();

        self.stake_farm(payments)
    }

    fn stake_farm(
        &self,
        payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> EnterFarmResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token().is_empty(), "No farm token");

        let payment_0 = payments
            .try_get(0)
            .unwrap_or_else(|| sc_panic!("empty payments"));
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let token_in = payment_0.token_identifier.clone();
        let enter_amount = payment_0.amount;

        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0u32, "Cannot farm with amount of 0");

        self.generate_aggregated_rewards();

        let attributes = StakingFarmTokenAttributes {
            reward_per_share: self.reward_per_share().get(),
            compounded_reward: BigUint::zero(),
            current_farm_amount: enter_amount.clone(),
        };

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token().get_token_id();
        let (new_farm_token, _created_with_merge) = self.create_farm_tokens_by_merging(
            farm_token_id,
            enter_amount,
            &attributes,
            &additional_payments,
        );
        self.send_tokens_non_zero(
            &caller,
            &new_farm_token.payment.token_identifier,
            new_farm_token.payment.token_nonce,
            &new_farm_token.payment.amount,
        );

        new_farm_token.payment
    }

    #[payable("*")]
    #[endpoint(unstakeFarm)]
    fn unstake_farm(&self) -> ExitFarmResultType<Self::Api> {
        let (payment_token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();
        let farm_token_id = self.farm_token().get_token_id();

        self.unstake_farm_common(farm_token_id, payment_token_id, token_nonce, amount, None)
    }

    #[payable("*")]
    #[endpoint(unstakeFarmThroughProxy)]
    fn unstake_farm_through_proxy(&self) -> ExitFarmResultType<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller);

        let payments = self.call_value().all_esdt_transfers();
        require!(payments.len() == 2, "Invalid payments amount");

        // first payment are the staking tokens, taken from the liquidity pool
        // they will be sent to the user on unbond
        let first_payment = payments.get(0);
        let staking_token_id = self.farming_token_id().get();
        require!(
            first_payment.token_identifier == staking_token_id,
            "Invalid first payment"
        );

        let second_payment = payments.get(1);
        let farm_token_id = self.farm_token().get_token_id();
        require!(
            second_payment.token_identifier == farm_token_id,
            "Invalid second payment"
        );

        self.unstake_farm_common(
            farm_token_id,
            second_payment.token_identifier,
            second_payment.token_nonce,
            second_payment.amount,
            Some(first_payment.amount),
        )
    }

    fn unstake_farm_common(
        &self,
        farm_token_id: TokenIdentifier,
        payment_token_id: TokenIdentifier,
        token_nonce: Nonce,
        payment_amount: BigUint,
        opt_unbond_amount: Option<BigUint>,
    ) -> ExitFarmResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(farm_token_id.is_valid_esdt_identifier(), "No farm token");

        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(payment_amount > 0u32, "Payment amount cannot be zero");

        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            token_nonce,
        );
        let reward_token_id = self.reward_token_id().get();
        self.generate_aggregated_rewards();

        let reward = self.calculate_reward(
            &payment_amount,
            &self.reward_per_share().get(),
            &farm_attributes.reward_per_share,
        );

        let caller = self.blockchain().get_caller();
        self.burn_farm_tokens(&payment_token_id, token_nonce, &payment_amount);

        let unbond_token_amount = match opt_unbond_amount {
            Some(amt) => amt,
            None => payment_amount, // payment_amount = initial_farming + compounded_rewards
        };
        let farm_token_payment =
            self.create_and_send_unbond_tokens(&caller, farm_token_id, unbond_token_amount);

        self.send_rewards(&reward_token_id, &reward, &caller);

        MultiValue2::from((
            farm_token_payment,
            EsdtTokenPayment::new(reward_token_id, 0, reward),
        ))
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

    #[payable("*")]
    #[endpoint(unbondFarm)]
    fn unbond_farm(&self) -> UnbondFarmResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token().is_empty(), "No farm token");

        let (payment_token_id, token_nonce, amount) = self.call_value().single_esdt().into_tuple();

        let farm_token_id = self.farm_token().get_token_id();
        require!(payment_token_id == farm_token_id, "Bad input token");
        require!(amount > 0, "Payment amount cannot be zero");

        let attributes: UnbondSftAttributes =
            self.get_farm_token_attributes(&farm_token_id, token_nonce);
        let current_epoch = self.blockchain().get_block_epoch();
        require!(
            current_epoch >= attributes.unlock_epoch,
            "Unbond period not over"
        );

        self.send()
            .esdt_local_burn(&farm_token_id, token_nonce, &amount);

        let caller = self.blockchain().get_caller();
        let farming_token_id = self.farming_token_id().get();
        self.send()
            .direct_esdt(&caller, &farming_token_id, 0, &amount);

        EsdtTokenPayment::new(farming_token_id, 0, amount)
    }

    #[payable("*")]
    #[endpoint(claimRewards)]
    fn claim_rewards(&self) -> ClaimRewardsResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token().is_empty(), "No farm token");

        let (payment_token_id, token_nonce, old_farming_amount) =
            self.call_value().single_esdt().into_tuple();
        require!(old_farming_amount > 0u32, "Zero amount");

        let farm_token_id = self.farm_token().get_token_id();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        let caller = self.blockchain().get_caller();
        self.claim_rewards_common(
            caller,
            farm_token_id,
            token_nonce,
            old_farming_amount.clone(),
            old_farming_amount,
        )
    }

    #[payable("*")]
    #[endpoint(claimRewardsWithNewValue)]
    fn claim_rewards_with_new_value(
        &self,
        new_farming_amount: BigUint,
    ) -> ClaimRewardsResultType<Self::Api> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token().is_empty(), "No farm token");

        let caller = self.blockchain().get_caller();
        self.require_whitelisted(&caller);

        let (payment_token_id, token_nonce, old_farming_amount) =
            self.call_value().single_esdt().into_tuple();
        require!(old_farming_amount > 0u32, "Zero amount");

        let farm_token_id = self.farm_token().get_token_id();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        self.claim_rewards_common(
            caller,
            farm_token_id,
            token_nonce,
            old_farming_amount,
            new_farming_amount,
        )
    }

    fn claim_rewards_common(
        &self,
        caller: ManagedAddress,
        farm_token_id: TokenIdentifier,
        farm_token_nonce: u64,
        old_farming_amount: BigUint,
        new_farming_amount: BigUint,
    ) -> ClaimRewardsResultType<Self::Api> {
        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &farm_token_id,
            farm_token_nonce,
        );

        self.generate_aggregated_rewards();

        let current_reward_per_share = self.reward_per_share().get();
        let reward = self.calculate_reward(
            &old_farming_amount,
            &current_reward_per_share,
            &farm_attributes.reward_per_share,
        );
        let new_compound_reward_amount = self.rule_of_three(
            &old_farming_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        );
        let new_attributes = StakingFarmTokenAttributes {
            reward_per_share: current_reward_per_share,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farming_amount.clone(),
        };

        self.burn_farm_tokens(&farm_token_id, farm_token_nonce, &old_farming_amount);

        let new_tokens = self.mint_farm_tokens(farm_token_id, new_farming_amount, &new_attributes);
        let new_farm_token = StakingFarmToken {
            payment: new_tokens,
            attributes: new_attributes,
        };

        self.send().direct_esdt(
            &caller,
            &new_farm_token.payment.token_identifier,
            new_farm_token.payment.token_nonce,
            &new_farm_token.payment.amount,
        );

        let reward_token_id = self.reward_token_id().get();
        self.send_rewards(&reward_token_id, &reward, &caller);

        (
            new_farm_token.payment,
            EsdtTokenPayment::new(reward_token_id, 0, reward),
        )
            .into()
    }

    #[payable("*")]
    #[endpoint(compoundRewards)]
    fn compound_rewards(&self) -> CompoundRewardsResultType<Self::Api> {
        require!(self.is_active(), "Not active");

        let payments_vec = self.call_value().all_esdt_transfers();
        let payment_0 = payments_vec
            .try_get(0)
            .unwrap_or_else(|| sc_panic!("empty payments"));
        let additional_payments = payments_vec
            .slice(1, payments_vec.len())
            .unwrap_or_default();

        let payment_token_id = payment_0.token_identifier.clone();
        let payment_amount = payment_0.amount.clone();
        let payment_token_nonce = payment_0.token_nonce;
        require!(payment_amount > 0u32, "Zero amount");

        require!(!self.farm_token().is_empty(), "No farm token");
        let farm_token_id = self.farm_token().get_token_id();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        let farming_token = self.farming_token_id().get();
        let reward_token = self.reward_token_id().get();
        require!(
            farming_token == reward_token,
            "Farming token differ from reward token"
        );
        self.generate_aggregated_rewards();

        let current_rps = self.reward_per_share().get();
        let farm_attributes = self.get_attributes::<StakingFarmTokenAttributes<Self::Api>>(
            &payment_token_id,
            payment_token_nonce,
        );
        let reward = self.calculate_reward(
            &payment_amount,
            &current_rps,
            &farm_attributes.reward_per_share,
        );

        let new_farm_contribution = &payment_amount + &reward;
        let new_compound_reward_amount = &self.rule_of_three(
            &payment_amount,
            &farm_attributes.current_farm_amount,
            &farm_attributes.compounded_reward,
        ) + &reward;

        let new_attributes = StakingFarmTokenAttributes {
            reward_per_share: current_rps,
            compounded_reward: new_compound_reward_amount,
            current_farm_amount: new_farm_contribution.clone(),
        };

        self.burn_farm_tokens(&farm_token_id, payment_token_nonce, &payment_amount);
        let caller = self.blockchain().get_caller();
        let (new_farm_token, _created_with_merge) = self.create_farm_tokens_by_merging(
            farm_token_id,
            new_farm_contribution,
            &new_attributes,
            &additional_payments,
        );
        self.send().direct_esdt(
            &caller,
            &new_farm_token.payment.token_identifier,
            new_farm_token.payment.token_nonce,
            &new_farm_token.payment.amount,
        );

        new_farm_token.payment
    }

    fn create_farm_tokens_by_merging(
        &self,
        farm_token_id: TokenIdentifier,
        amount: BigUint,
        attributes: &StakingFarmTokenAttributes<Self::Api>,
        additional_payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> (StakingFarmToken<Self::Api>, bool) {
        let current_position_replic = StakingFarmToken {
            payment: EsdtTokenPayment::new(farm_token_id.clone(), 0, amount),
            attributes: attributes.clone(),
        };

        let additional_payments_len = additional_payments.len();
        let merged_attributes = self
            .get_merged_farm_token_attributes(additional_payments, Some(current_position_replic));
        self.burn_farm_tokens_from_payments(additional_payments);

        let new_tokens = self.mint_farm_tokens(
            farm_token_id,
            merged_attributes.current_farm_amount.clone(),
            &merged_attributes,
        );
        let new_farm_token = StakingFarmToken {
            payment: new_tokens,
            attributes: merged_attributes,
        };
        let is_merged = additional_payments_len != 0;

        (new_farm_token, is_merged)
    }

    fn send_rewards(
        &self,
        reward_token_id: &TokenIdentifier,
        reward_amount: &BigUint,
        destination: &ManagedAddress,
    ) {
        self.send_tokens_non_zero(destination, reward_token_id, 0, reward_amount);
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        amount: BigUint,
        attributes: StakingFarmTokenAttributes<Self::Api>,
    ) -> BigUint {
        require!(amount > 0, "Zero liquidity input");
        let farm_token_supply = self.farm_token_supply().get();
        require!(farm_token_supply >= amount, "Not enough supply");

        let last_reward_nonce = self.last_reward_block_nonce().get();
        let current_block_nonce = self.blockchain().get_block_nonce();
        let reward_increase =
            self.calculate_per_block_rewards(current_block_nonce, last_reward_nonce);
        let reward_per_share_increase =
            self.calculate_reward_per_share_increase(&reward_increase, &farm_token_supply);

        let future_reward_per_share = self.reward_per_share().get() + reward_per_share_increase;

        self.calculate_reward(
            &amount,
            &future_reward_per_share,
            &attributes.reward_per_share,
        )
    }
}
