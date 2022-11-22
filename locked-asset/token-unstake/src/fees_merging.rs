elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use energy_factory::token_merging::LockedAmountWeightAttributesPair;
use mergeable::Mergeable;
use simple_lock::locked_token::LockedTokenAttributes;

static LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"lockedTokenId";

#[elrond_wasm::module]
pub trait FeesMergingModule:
    energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
    + utils::UtilsModule
{
    fn merge_fees_from_penalty(&self, payment: EsdtTokenPayment) {
        let fee_mapper = self.fees_from_penalty_unlocking();
        let mut output_pair = fee_mapper.get();

        let new_token_attributes: LockedTokenAttributes<Self::Api> =
            self.get_token_attributes(&payment.token_identifier, payment.token_nonce);
        let new_pair = LockedAmountWeightAttributesPair::new(payment.amount, new_token_attributes);
        output_pair.merge_with(new_pair);

        let normalized_unlock_epoch =
            self.unlock_epoch_to_start_of_month_upper_estimate(output_pair.attributes.unlock_epoch);
        output_pair.attributes.unlock_epoch = normalized_unlock_epoch;

        fee_mapper.set(&output_pair);
    }

    fn get_locked_token_id(&self, energy_factory_addr: &ManagedAddress) -> TokenIdentifier {
        self.storage_raw().read_from_address(
            energy_factory_addr,
            ManagedBuffer::new_from_bytes(LOCKED_TOKEN_ID_STORAGE_KEY),
        )
    }

    #[storage_mapper("feesFromPenaltyUnlocking")]
    fn fees_from_penalty_unlocking(
        &self,
    ) -> SingleValueMapper<LockedAmountWeightAttributesPair<Self::Api>>;
}
