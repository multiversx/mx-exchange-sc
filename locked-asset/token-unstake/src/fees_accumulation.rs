elrond_wasm::imports!();

use common_structs::Epoch;
use energy_factory::lock_options::MAX_PENALTY_PERCENTAGE;
use week_timekeeping::EPOCHS_IN_WEEK;

static LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"lockedTokenId";
static BASE_TOKEN_ID_STORAGE_KEY: &[u8] = b"baseAssetTokenId";

use crate::{events, tokens_per_user::UnstakePair};

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
    crate::tokens_per_user::TokensPerUserModule
    + energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
    + locking_module::lock_with_energy_module::LockWithEnergyModule
    + energy_query::EnergyQueryModule
    + utils::UtilsModule
    + events::EventsModule
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

        let new_unlocked_tokens = self.unlocked_tokens_for_user(&user).get();
        self.emit_unlocked_tokens_event(&user, new_unlocked_tokens);
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
        let locked_token_id =
            self.get_locked_token_id(&energy_factory_addr, LOCKED_TOKEN_ID_STORAGE_KEY);
        require!(payment.token_identifier == locked_token_id, "Invalid token");

        self.burn_penalty(payment);
    }

    fn burn_penalty(&self, payment: EsdtTokenPayment) {
        let fees_burn_percentage = self.fees_burn_percentage().get();
        let burn_amount = &payment.amount * fees_burn_percentage / MAX_PENALTY_PERCENTAGE;
        let remaining_amount = &payment.amount - &burn_amount;
        if remaining_amount > 0 {
            self.fees_from_penalty_unlocking()
                .update(|fees| *fees += remaining_amount)
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
        let total_fees = fees_mapper.get();
        if total_fees == 0u64 {
            last_send_mapper.set(current_epoch);

            return;
        }

        let energy_factory_addr = self.energy_factory_address().get();

        let fee_tokens: EsdtTokenPayment = self.lock_virtual(
            self.get_locked_token_id(&energy_factory_addr, BASE_TOKEN_ID_STORAGE_KEY),
            total_fees,
            self.blockchain().get_sc_address(),
            energy_factory_addr,
        );

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

    fn get_locked_token_id(
        &self,
        energy_factory_addr: &ManagedAddress,
        token_key: &[u8],
    ) -> TokenIdentifier {
        self.storage_raw().read_from_address(
            energy_factory_addr,
            ManagedBuffer::new_from_bytes(token_key),
        )
    }

    #[proxy]
    fn fees_collector_proxy_builder(
        &self,
        sc_address: ManagedAddress,
    ) -> fees_collector_proxy::Proxy<Self::Api>;

    #[view(getFeesFromPenaltyUnlocking)]
    #[storage_mapper("feesFromPenaltyUnlocking")]
    fn fees_from_penalty_unlocking(&self) -> SingleValueMapper<BigUint>;

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
