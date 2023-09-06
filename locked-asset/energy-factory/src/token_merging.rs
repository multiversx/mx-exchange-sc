multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use multiversx_sc::api::BlockchainApi;

use common_structs::PaymentsVec;
use math::weighted_average_round_up;
use mergeable::Mergeable;
use simple_lock::locked_token::LockedTokenAttributes;
use unwrappable::Unwrappable;

use crate::{energy::Energy, unlock_with_penalty::TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG};

#[derive(TopEncode, TopDecode, Clone, PartialEq, Debug)]
pub struct LockedAmountWeightAttributesPair<M: ManagedTypeApi> {
    pub token_amount: BigUint<M>,
    pub attributes: LockedTokenAttributes<M>,
}

impl<M: ManagedTypeApi> LockedAmountWeightAttributesPair<M> {
    pub fn new(token_amount: BigUint<M>, attributes: LockedTokenAttributes<M>) -> Self {
        Self {
            token_amount,
            attributes,
        }
    }
}

impl<M: ManagedTypeApi + BlockchainApi> Mergeable<M> for LockedAmountWeightAttributesPair<M> {
    fn can_merge_with(&self, other: &Self) -> bool {
        let same_token_id = self.attributes.original_token_id == other.attributes.original_token_id;
        let same_token_nonce =
            self.attributes.original_token_nonce == other.attributes.original_token_nonce;

        same_token_id && same_token_nonce
    }

    fn merge_with(&mut self, other: Self) {
        self.error_if_not_mergeable(&other);

        let new_unlock_epoch = weighted_average_round_up(
            BigUint::from(self.attributes.unlock_epoch),
            self.token_amount.clone(),
            BigUint::from(other.attributes.unlock_epoch),
            other.token_amount.clone(),
        )
        .to_u64()
        .unwrap_or_panic::<M>();

        self.attributes.unlock_epoch = new_unlock_epoch;
        self.token_amount += other.token_amount;
    }
}

#[multiversx_sc::module]
pub trait TokenMergingModule:
    simple_lock::basic_lock_unlock::BasicLockUnlock
    + simple_lock::locked_token::LockedTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + simple_lock::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::pause::PauseModule
    + crate::penalty::LocalPenaltyModule
    + crate::energy::EnergyModule
    + crate::events::EventsModule
    + crate::lock_options::LockOptionsModule
    + utils::UtilsModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[payable("*")]
    #[endpoint(mergeTokens)]
    fn merge_tokens_endpoint(
        &self,
        opt_original_caller: OptionalValue<ManagedAddress>,
    ) -> EsdtTokenPayment {
        self.require_not_paused();

        let payments = self.get_non_empty_payments();
        let caller = self.blockchain().get_caller();
        let original_caller = self.get_orig_caller_from_opt(&caller, opt_original_caller);

        let output_amount_attributes = self.update_energy(&original_caller, |energy| {
            self.merge_tokens(payments, energy)
        });
        let simulated_lock_payment = EgldOrEsdtTokenPayment::new(
            output_amount_attributes.attributes.original_token_id,
            output_amount_attributes.attributes.original_token_nonce,
            output_amount_attributes.token_amount,
        );
        let output_tokens = self.lock_and_send(
            &caller,
            simulated_lock_payment,
            output_amount_attributes.attributes.unlock_epoch,
        );

        self.to_esdt_payment(output_tokens)
    }

    fn merge_tokens(
        self,
        mut payments: PaymentsVec<Self::Api>,
        energy: &mut Energy<Self::Api>,
    ) -> LockedAmountWeightAttributesPair<Self::Api> {
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

        energy.update_after_unlock_any(
            &first_payment.amount,
            first_token_attributes.unlock_epoch,
            current_epoch,
        );

        locked_token_mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        let mut output_pair =
            LockedAmountWeightAttributesPair::new(first_payment.amount, first_token_attributes);
        for payment in &payments {
            let attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(payment.token_nonce);
            require!(
                attributes.unlock_epoch > current_epoch,
                TOKEN_CAN_BE_UNLOCKED_ALREADY_ERR_MSG
            );

            energy.update_after_unlock_any(&payment.amount, attributes.unlock_epoch, current_epoch);

            locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

            let amount_attr_pair =
                LockedAmountWeightAttributesPair::new(payment.amount, attributes);
            output_pair.merge_with(amount_attr_pair);
        }

        let normalized_unlock_epoch =
            self.unlock_epoch_to_start_of_month_upper_estimate(output_pair.attributes.unlock_epoch);
        output_pair.attributes.unlock_epoch = normalized_unlock_epoch;

        energy.add_after_token_lock(
            &output_pair.token_amount,
            output_pair.attributes.unlock_epoch,
            current_epoch,
        );

        output_pair
    }
}
