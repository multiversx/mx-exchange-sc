elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use energy_factory::token_merging::LockedAmountWeightAttributesPair;
use mergeable::Mergeable;
use simple_lock::locked_token::LockedTokenAttributes;

static LOCKED_TOKEN_ID_STORAGE_KEY: &[u8] = b"lockedTokenId";

#[derive(TopEncode, TopDecode, PartialEq, Debug)]
pub struct EncodabLockedAmountWeightAttributesPair<M: ManagedTypeApi> {
    pub token_amount: BigUint<M>,
    pub token_unlock_fee_percent: u64,
    pub attributes: LockedTokenAttributes<M>,
}

impl<M: ManagedTypeApi> EncodabLockedAmountWeightAttributesPair<M> {
    pub fn from_ref_instance<'a, Sc>(ref_instance: LockedAmountWeightAttributesPair<Sc>) -> Self
    where
        Sc: energy_factory::penalty::LocalPenaltyModule<Api = M>,
    {
        EncodabLockedAmountWeightAttributesPair {
            token_amount: ref_instance.token_amount,
            token_unlock_fee_percent: ref_instance.token_unlock_fee_percent,
            attributes: ref_instance.attributes,
        }
    }

    pub fn into_self_ref_instance<'a, Sc>(
        self,
        sc_ref: &'a Sc,
    ) -> LockedAmountWeightAttributesPair<Sc>
    where
        Sc: energy_factory::penalty::LocalPenaltyModule<Api = M>,
    {
        LockedAmountWeightAttributesPair {
            sc_ref,
            attributes: self.attributes,
            token_amount: self.token_amount,
            token_unlock_fee_percent: self.token_unlock_fee_percent,
        }
    }
}

#[elrond_wasm::module]
pub trait FeesMergingModule:
    energy_factory::penalty::LocalPenaltyModule
    + energy_factory::lock_options::LockOptionsModule
    + utils::UtilsModule
{
    fn merge_fees_from_penalty(&self, payment: EsdtTokenPayment) {
        let fee_mapper = self.fees_from_penalty_unlocking();
        let stored_entry = fee_mapper.get();
        let mut output_pair = stored_entry.into_self_ref_instance(self);

        let new_token_attributes: LockedTokenAttributes<Self::Api> =
            self.get_token_attributes(&payment.token_identifier, payment.token_nonce);
        let new_pair =
            LockedAmountWeightAttributesPair::new(self, payment.amount, new_token_attributes);
        output_pair.merge_with(new_pair);

        let normalized_unlock_epoch =
            self.unlock_epoch_to_start_of_month_upper_estimate(output_pair.attributes.unlock_epoch);
        output_pair.attributes.unlock_epoch = normalized_unlock_epoch;

        let encodable_instance =
            EncodabLockedAmountWeightAttributesPair::from_ref_instance(output_pair);
        fee_mapper.set(&encodable_instance);
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
    ) -> SingleValueMapper<EncodabLockedAmountWeightAttributesPair<Self::Api>>;
}
