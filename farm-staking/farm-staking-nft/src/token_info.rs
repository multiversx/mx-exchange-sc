use common_structs::{Nonce, PaymentAttributesPair, PaymentsVec};
use mergeable::Mergeable;

use crate::token_attributes::{
    PartialStakingFarmNftTokenAttributes, StakingFarmNftTokenAttributes,
};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait TokenInfoModule: utils::UtilsModule {
    fn into_part(
        &self,
        attributes: StakingFarmNftTokenAttributes<Self::Api>,
        payment: &EsdtTokenPayment,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        let total_supply = self.total_supply(payment.token_nonce).get();
        if payment.amount == total_supply {
            self.remaining_supply(payment.token_nonce).clear();
            self.remaining_parts(payment.token_nonce).clear();

            return PartialStakingFarmNftTokenAttributes {
                reward_per_share: attributes.reward_per_share,
                compounded_reward: attributes.compounded_reward,
                original_owner: attributes.original_owner,
                farming_token_parts: attributes.farming_token_parts,
                current_farm_amount: payment.amount.clone(),
            };
        }

        let new_compounded_reward = self.rule_of_three(
            &total_supply,
            &payment.amount,
            &attributes.compounded_reward,
        );
        let remaining_parts = self.get_token_parts(payment, &total_supply);
        let new_current_farm_amount = self.get_new_total_tokens(&remaining_parts);
        require!(new_current_farm_amount > 0, "Could not split token");

        PartialStakingFarmNftTokenAttributes {
            reward_per_share: attributes.reward_per_share,
            compounded_reward: new_compounded_reward,
            original_owner: attributes.original_owner,
            farming_token_parts: remaining_parts,
            current_farm_amount: new_current_farm_amount,
        }
    }

    fn get_token_parts(
        &self,
        payment: &EsdtTokenPayment,
        total_supply: &BigUint,
    ) -> PaymentsVec<Self::Api> {
        let remaining_supply_mapper = self.remaining_supply(payment.token_nonce);
        let remaining_parts_mapper = self.remaining_parts(payment.token_nonce);

        let remaining_supply = remaining_supply_mapper.get();
        let token_parts = if remaining_supply != payment.amount {
            remaining_parts_mapper.update(|rem_parts| {
                let mut i = 0;
                let mut max_len = rem_parts.len();
                let mut token_parts = ManagedVec::new();
                while i < max_len {
                    let max_token_parts = rem_parts.get(i);
                    let new_amount =
                        self.rule_of_three(total_supply, &payment.amount, &max_token_parts.amount);
                    if new_amount > 0 {
                        let new_amount_payment = EsdtTokenPayment::new(
                            max_token_parts.token_identifier.clone(),
                            max_token_parts.token_nonce,
                            new_amount.clone(),
                        );
                        token_parts.push(new_amount_payment);
                    }

                    let remaining_part_amount = max_token_parts.amount - new_amount;
                    if remaining_part_amount > 0 {
                        let rem_payment = EsdtTokenPayment::new(
                            max_token_parts.token_identifier,
                            max_token_parts.token_nonce,
                            remaining_part_amount,
                        );
                        let _ = rem_parts.set(i, &rem_payment);

                        i += 1;
                    } else {
                        // else branch might not be needed?
                        rem_parts.remove(i);
                        max_len -= 1;
                    }
                }

                require!(!rem_parts.is_empty(), "May not split this token");

                token_parts
            })
        } else {
            remaining_parts_mapper.take()
        };

        remaining_supply_mapper.set(&remaining_supply - &payment.amount);

        token_parts
    }

    fn get_new_total_tokens(&self, tokens: &PaymentsVec<Self::Api>) -> BigUint {
        let mut total = BigUint::zero();
        for token in tokens {
            total += token.amount;
        }

        total
    }

    fn get_attributes_as_part_of_fixed_supply_nft(
        &self,
        payment: &EsdtTokenPayment,
        mapper: &NonFungibleTokenMapper,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        let attr: StakingFarmNftTokenAttributes<Self::Api> =
            mapper.get_token_attributes(payment.token_nonce);
        self.into_part(attr, payment)
    }

    fn merge_from_payments_and_burn_nft(
        &self,
        mut payments: PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        let first_payment = self.pop_first_payment(&mut payments);
        let base_attributes =
            self.get_attributes_as_part_of_fixed_supply_nft(&first_payment, mapper);
        mapper.nft_burn(first_payment.token_nonce, &first_payment.amount);

        let output_attributes =
            self.merge_attributes_from_payments_nft(base_attributes, &payments, mapper);
        self.send().esdt_local_burn_multi(&payments);

        output_attributes
    }

    fn merge_attributes_from_payments_nft(
        &self,
        mut base_attributes: PartialStakingFarmNftTokenAttributes<Self::Api>,
        payments: &PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> PartialStakingFarmNftTokenAttributes<Self::Api> {
        for payment in payments {
            let attributes = self.get_attributes_as_part_of_fixed_supply_nft(&payment, mapper);
            base_attributes.merge_with(attributes);
        }

        base_attributes
    }

    fn merge_and_create_token_nft(
        &self,
        base_attributes: PartialStakingFarmNftTokenAttributes<Self::Api>,
        payments: &PaymentsVec<Self::Api>,
        mapper: &NonFungibleTokenMapper,
    ) -> PaymentAttributesPair<Self::Api, PartialStakingFarmNftTokenAttributes<Self::Api>> {
        let output_attributes =
            self.merge_attributes_from_payments_nft(base_attributes, payments, mapper);
        let new_token_amount = output_attributes.current_farm_amount.clone();
        let new_token_payment = mapper.nft_create(new_token_amount, &output_attributes);

        PaymentAttributesPair {
            payment: new_token_payment,
            attributes: output_attributes,
        }
    }

    /// full_value * current_supply / total_supply
    fn rule_of_three(
        &self,
        total_supply: &BigUint,
        current_supply: &BigUint,
        full_value: &BigUint,
    ) -> BigUint {
        if current_supply == total_supply {
            return full_value.clone();
        }

        (full_value * current_supply) / total_supply
    }

    #[storage_mapper("totalSupply")]
    fn total_supply(&self, token_nonce: Nonce) -> SingleValueMapper<BigUint>;

    #[storage_mapper("remainingSupply")]
    fn remaining_supply(&self, token_nonce: Nonce) -> SingleValueMapper<BigUint>;

    #[storage_mapper("remainingParts")]
    fn remaining_parts(&self, token_nonce: Nonce) -> SingleValueMapper<PaymentsVec<Self::Api>>;
}
