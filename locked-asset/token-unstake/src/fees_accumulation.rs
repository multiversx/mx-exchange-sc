elrond_wasm::imports!();

use common_structs::Epoch;
use energy_factory::{
    lock_options::MAX_PENALTY_PERCENTAGE, token_merging::LockedAmountWeightAttributesPair,
    unstake::ProxyTrait as _,
};
use simple_lock::locked_token::LockedTokenAttributes;
use week_timekeeping::EPOCHS_IN_WEEK;

use crate::{fees_merging::EncodabLockedAmountWeightAttributesPair, tokens_per_user::UnstakePair};

pub mod fees_collector_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait FeesCollectorProxy {
        #[payable("*")]
        #[endpoint(depositSwapFees)]
        fn deposit_swap_fees(&self);
    }
}

#[elrond_wasm::module]
pub trait FeesAccumulationModule:
    crate::fees_merging::FeesMergingModule
    + crate::tokens_per_user::TokensPerUserModule
    + energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
{
    #[payable("*")]
    #[endpoint(depositUserTokens)]
    fn deposit_user_tokens(&self, user: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        let energy_factory_address = self.energy_factory_address().get();
        require!(
            caller == energy_factory_address,
            "Only energy factory SC can call this endpoint"
        );

        let [locked_tokens, unlocked_tokens] = self.call_value().multi_esdt();
        let current_epoch = self.blockchain().get_block_epoch();
        let unbond_epochs = self.unbond_epochs().get();
        let unlock_epoch = current_epoch + unbond_epochs;
        self.unlocked_tokens_for_user(&user)
            .update(|unstake_pairs| {
                let unstake_pair = UnstakePair {
                    unlock_epoch,
                    locked_tokens,
                    unlocked_tokens,
                };
                unstake_pairs.push(unstake_pair);
            });

        self.send_fees_to_collector();
    }

    #[payable("*")]
    #[endpoint(depositFees)]
    fn deposit_fees(&self) {
        let energy_factory_addr = self.energy_factory_address().get();
        let caller = self.blockchain().get_caller();
        require!(
            caller == energy_factory_addr,
            "Only energy factory may deposit fees"
        );

        let payment = self.call_value().single_esdt();
        let locked_token_id = self.get_locked_token_id(&energy_factory_addr);
        require!(payment.token_identifier == locked_token_id, "Invalid token");

        self.burn_penalty(payment);
    }

    fn burn_penalty(&self, payment: EsdtTokenPayment) {
        let fees_burn_percentage = self.fees_burn_percentage().get();
        let burn_amount = &payment.amount * fees_burn_percentage / MAX_PENALTY_PERCENTAGE;
        let remaining_amount = &payment.amount - &burn_amount;
        if remaining_amount > 0 {
            let fees_mapper = self.fees_from_penalty_unlocking();
            if !fees_mapper.is_empty() {
                self.merge_fees_from_penalty(EsdtTokenPayment::new(
                    payment.token_identifier.clone(),
                    payment.token_nonce,
                    remaining_amount,
                ));
            } else {
                let token_attributes: LockedTokenAttributes<Self::Api> =
                    self.get_token_attributes(&payment.token_identifier, payment.token_nonce);
                let ref_instance =
                    LockedAmountWeightAttributesPair::new(self, remaining_amount, token_attributes);
                let encodable_instance =
                    EncodabLockedAmountWeightAttributesPair::from_ref_instance(ref_instance);

                fees_mapper.set(&encodable_instance);
            }
        }

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        // Only once per week
        self.send_fees_to_collector();
    }

    fn send_fees_to_collector(&self) {
        let last_send_mapper = self.last_epoch_fee_sent_to_collector();
        let current_epoch = self.blockchain().get_block_epoch();
        let last_epoch_fee_sent_to_collector = last_send_mapper.get();
        let next_send_epoch = last_epoch_fee_sent_to_collector + EPOCHS_IN_WEEK;
        if current_epoch < next_send_epoch {
            return;
        }

        let fees_mapper = self.fees_from_penalty_unlocking();
        if fees_mapper.is_empty() {
            last_send_mapper.set(current_epoch);

            return;
        }

        let fees_attributes = fees_mapper.get();
        let energy_factory_addr = self.energy_factory_address().get();
        let fee_tokens: EsdtTokenPayment = self
            .energy_factory_proxy(energy_factory_addr)
            .create_merged_locked_token_for_fees(
                fees_attributes.token_amount,
                fees_attributes.attributes.unlock_epoch,
            )
            .execute_on_dest_context();

        let fees_collector_addr = self.fees_collector_address().get();
        let _: IgnoreValue = self
            .fees_collector_proxy_builder(fees_collector_addr)
            .deposit_swap_fees()
            .add_esdt_token_transfer(
                fee_tokens.token_identifier,
                fee_tokens.token_nonce,
                fee_tokens.amount,
            )
            .execute_on_dest_context();

        fees_mapper.clear();
    }

    #[proxy]
    fn fees_collector_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> fees_collector_proxy::Proxy<Self::Api>;

    #[view(getFeesBurnPercentage)]
    #[storage_mapper("feesBurnPercentage")]
    fn fees_burn_percentage(&self) -> SingleValueMapper<u64>;

    #[view(getFeesCollectorAddress)]
    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getLastEpochFeeSentToCollector)]
    #[storage_mapper("lastEpochFeeSentToCollector")]
    fn last_epoch_fee_sent_to_collector(&self) -> SingleValueMapper<Epoch>;
}
