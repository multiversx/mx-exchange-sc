elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use energy_factory::token_merging::LockedAmountWeightAttributesPair;
use simple_lock::locked_token::LockedTokenAttributes;

#[derive(TopEncode, TopDecode)]
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

    pub fn to_self_ref_instance<'a, Sc>(
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

// TODO: Cleanup modules to not duplicate endpoints in this SC
#[elrond_wasm::module]
pub trait FeesMergingModule:
    energy_factory::penalty::LocalPenaltyModule + energy_factory::lock_options::LockOptionsModule
{
    fn add_and_merge_fees_after_unbond(&self) {}

    /// Merges new fees with existing fees and saves in storage
    fn merge_fees_from_penalty(&self, token_nonce: Nonce, new_fee_amount: BigUint) {
        let locked_token_mapper = self.locked_token();
        let existing_nonce_amount_pair = self.fees_from_penalty_unlocking().get();
        let mut payments = PaymentsVec::new();
        payments.push(EsdtTokenPayment::new(
            locked_token_mapper.get_token_id(),
            token_nonce,
            new_fee_amount,
        ));
        payments.push(EsdtTokenPayment::new(
            locked_token_mapper.get_token_id(),
            existing_nonce_amount_pair.nonce,
            existing_nonce_amount_pair.amount,
        ));

        let new_locked_amount_attributes = self.merge_tokens(payments, &mut None);

        let sft_nonce = self.get_or_create_nonce_for_attributes(
            &locked_token_mapper,
            &new_locked_amount_attributes
                .attributes
                .original_token_id
                .clone()
                .into_name(),
            &new_locked_amount_attributes.attributes,
        );

        let new_locked_tokens = locked_token_mapper
            .nft_add_quantity(sft_nonce, new_locked_amount_attributes.token_amount);

        self.fees_from_penalty_unlocking().set(NonceAmountPair::new(
            new_locked_tokens.token_nonce,
            new_locked_tokens.amount,
        ));
    }

    fn merge_tokens(
        self,
        mut payments: PaymentsVec<Self::Api>,
    ) -> LockedAmountWeightAttributesPair<Self> {
        let locked_token_mapper = self.locked_token();
        locked_token_mapper.require_all_same_token(&payments);

        let first_payment = payments.get(0);
        payments.remove(0);

        let current_epoch = self.blockchain().get_block_epoch();
        let first_token_attributes: LockedTokenAttributes<Self::Api> =
            locked_token_mapper.get_token_attributes(first_payment.token_nonce);
        require!(
            first_token_attributes.unlock_epoch > current_epoch,
            TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
        );

        locked_token_mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        let mut output_pair = LockedAmountWeightAttributesPair::new(
            self,
            first_payment.amount,
            first_token_attributes,
        );
        for payment in &payments {
            let attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(payment.token_nonce);
            require!(
                attributes.unlock_epoch > current_epoch,
                TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
            );

            locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

            let amount_attr_pair =
                LockedAmountWeightAttributesPair::new(self, payment.amount, attributes);
            output_pair.merge_with(amount_attr_pair);
        }

        let normalized_unlock_epoch =
            self.unlock_epoch_to_start_of_month_upper_estimate(output_pair.attributes.unlock_epoch);
        output_pair.attributes.unlock_epoch = normalized_unlock_epoch;

        output_pair
    }

    #[storage_mapper("feesFromPenaltyUnlocking")]
    fn fees_from_penalty_unlocking(
        &self,
    ) -> SingleValueMapper<EncodabLockedAmountWeightAttributesPair<Self::Api>>;
}
