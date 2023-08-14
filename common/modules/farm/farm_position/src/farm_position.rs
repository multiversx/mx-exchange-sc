#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::FarmTokenAttributes;

#[multiversx_sc::module]
pub trait FarmPositionModule:
    farm_token::FarmTokenModule
    + utils::UtilsModule
    + permissions_module::PermissionsModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("*")]
    #[endpoint(updateTotalFarmPosition)]
    fn update_total_farm_position(&self) {
        let caller = self.blockchain().get_caller();
        let payments = self.get_non_empty_payments();
        let farm_token_mapper = self.farm_token();
        let farm_token_id = farm_token_mapper.get_token_id();
        let mut new_total_farm_position = BigUint::zero();
        for farm_position in &payments {
            require!(
                farm_position.token_identifier == farm_token_id,
                "Bad payment token"
            );
            let token_attributes: FarmTokenAttributes<Self::Api> =
                farm_token_mapper.get_token_attributes(farm_position.token_nonce);

            if &token_attributes.original_owner != &caller {
                self.user_total_farm_position(&token_attributes.original_owner)
                    .update(|user_farm_position| {
                        if *user_farm_position > farm_position.amount {
                            *user_farm_position -= &farm_position.amount;
                        } else {
                            *user_farm_position = BigUint::zero();
                        }
                    });
            }

            new_total_farm_position += farm_position.amount;
        }

        let user_current_farm_position = self.user_total_farm_position(&caller).get();
        if new_total_farm_position > user_current_farm_position {
            self.user_total_farm_position(&caller)
                .set(new_total_farm_position)
        }
    }

    fn check_and_update_user_farm_position(
        &self,
        user: &ManagedAddress,
        farm_position: &EsdtTokenPayment,
    ) {
        let farm_token_mapper = self.farm_token();
        let token_attributes: FarmTokenAttributes<Self::Api> =
            farm_token_mapper.get_token_attributes(farm_position.token_nonce);

        if &token_attributes.original_owner != user {
            self.user_total_farm_position(&token_attributes.original_owner)
                .update(|user_farm_position| {
                    if *user_farm_position > farm_position.amount {
                        *user_farm_position -= &farm_position.amount;
                    } else {
                        *user_farm_position = BigUint::zero();
                    }
                });

            self.user_total_farm_position(user)
                .update(|user_farm_position| *user_farm_position += &farm_position.amount);
        }
    }

    fn increase_user_farm_position(
        &self,
        user: &ManagedAddress,
        new_farm_position_amount: &BigUint,
    ) {
        self.user_total_farm_position(user)
            .update(|user_farm_position| *user_farm_position += new_farm_position_amount);
    }

    fn decrease_user_farm_position(&self, farm_position: &EsdtTokenPayment) {
        let farm_token_mapper = self.farm_token();
        let token_attributes: FarmTokenAttributes<Self::Api> =
            farm_token_mapper.get_token_attributes(farm_position.token_nonce);

        self.user_total_farm_position(&token_attributes.original_owner)
            .update(|user_farm_position| {
                if *user_farm_position > farm_position.amount {
                    *user_farm_position -= &farm_position.amount;
                } else {
                    *user_farm_position = BigUint::zero();
                }
            });
    }

    #[view(getUserTotalFarmPosition)]
    #[storage_mapper("userTotalFarmPosition")]
    fn user_total_farm_position(&self, user: &ManagedAddress) -> SingleValueMapper<BigUint>;
}
