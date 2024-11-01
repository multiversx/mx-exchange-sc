use common_structs::FarmTokenAttributes;

use crate::{
    dual_yield_token::DualYieldTokenAttributes,
    result_types::{ClaimDualYieldResult, StakeProxyResult},
};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ProxyExternalInteractionsModule:
    crate::dual_yield_token::DualYieldTokenModule
    + crate::external_contracts_interactions::ExternalContractsInteractionsModule
    + crate::lp_farm_token::LpFarmTokenModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
    + utils::UtilsModule
    + token_send::TokenSendModule
    + energy_query::EnergyQueryModule
    + sc_whitelist_module::SCWhitelistModule
{
    #[payable("*")]
    #[endpoint(stakeFarmOnBehalf)]
    fn stake_farm_on_behalf(&self, original_owner: ManagedAddress) -> StakeProxyResult<Self::Api> {
        let caller = self.blockchain().get_caller();
        self.require_user_whitelisted(&original_owner, &caller);

        let payments = self.get_non_empty_payments();
        let lp_farm_token_payment = payments.get(0);
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        self.check_stake_farm_payments(
            &original_owner,
            &lp_farm_token_payment,
            &additional_payments,
        );
        let lp_farm_token_id = self.lp_farm_token_id().get();
        require!(
            lp_farm_token_payment.token_identifier == lp_farm_token_id,
            "Invalid first payment"
        );

        let dual_yield_token_mapper = self.dual_yield_token();
        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut additional_staking_farm_tokens = ManagedVec::new();
        let mut additional_lp_farm_tokens = ManagedVec::new();
        for p in &additional_payments {
            let attributes: DualYieldTokenAttributes<Self::Api> =
                self.get_attributes_as_part_of_fixed_supply(&p, &dual_yield_token_mapper);

            additional_staking_farm_tokens.push(EsdtTokenPayment::new(
                staking_farm_token_id.clone(),
                attributes.staking_farm_token_nonce,
                attributes.staking_farm_token_amount,
            ));

            additional_lp_farm_tokens.push(EsdtTokenPayment::new(
                lp_farm_token_payment.token_identifier.clone(),
                attributes.lp_farm_token_nonce,
                attributes.lp_farm_token_amount,
            ));

            dual_yield_token_mapper.nft_burn(p.token_nonce, &p.amount);
        }

        let lp_tokens_in_farm = self.get_lp_tokens_in_farm_position(
            lp_farm_token_payment.token_nonce,
            &lp_farm_token_payment.amount,
        );
        let staking_token_amount = self.get_lp_tokens_safe_price(lp_tokens_in_farm);
        let staking_farm_enter_result = self.staking_farm_enter(
            original_owner.clone(),
            staking_token_amount,
            additional_staking_farm_tokens,
        );
        let received_staking_farm_token = staking_farm_enter_result.received_staking_farm_token;

        let (merged_lp_farm_tokens, lp_farm_boosted_rewards) = self
            .merge_lp_farm_tokens(
                original_owner.clone(),
                lp_farm_token_payment,
                additional_lp_farm_tokens,
            )
            .into_tuple();

        let new_attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce: merged_lp_farm_tokens.token_nonce,
            lp_farm_token_amount: merged_lp_farm_tokens.amount,
            staking_farm_token_nonce: received_staking_farm_token.token_nonce,
            staking_farm_token_amount: received_staking_farm_token.amount,
        };
        let new_dual_yield_tokens =
            self.create_dual_yield_tokens(&dual_yield_token_mapper, &new_attributes);
        let output_payments = StakeProxyResult {
            dual_yield_tokens: new_dual_yield_tokens,
            staking_boosted_rewards: staking_farm_enter_result.boosted_rewards,
            lp_farm_boosted_rewards,
        };

        self.send_payment_non_zero(&original_owner, &output_payments.lp_farm_boosted_rewards);
        self.send_payment_non_zero(&original_owner, &output_payments.staking_boosted_rewards);
        self.send_payment_non_zero(&caller, &output_payments.dual_yield_tokens);

        output_payments
    }

    #[payable("*")]
    #[endpoint(claimDualYieldOnBehalf)]
    fn claim_dual_yield_on_behalf(&self) -> ClaimDualYieldResult<Self::Api> {
        let caller = self.blockchain().get_caller();

        let payment = self.call_value().single_esdt();
        let dual_yield_token_mapper = self.dual_yield_token();
        dual_yield_token_mapper.require_same_token(&payment.token_identifier);

        let attributes: DualYieldTokenAttributes<Self::Api> =
            self.get_attributes_as_part_of_fixed_supply(&payment, &dual_yield_token_mapper);

        let original_owner = self.get_farm_position_original_owner(attributes.lp_farm_token_nonce);
        self.require_user_whitelisted(&original_owner, &caller);

        let lp_tokens_in_position = self.get_lp_tokens_in_farm_position(
            attributes.lp_farm_token_nonce,
            &attributes.lp_farm_token_amount,
        );
        let new_staking_farm_value = self.get_lp_tokens_safe_price(lp_tokens_in_position);

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let lp_farm_claim_rewards_result = self.lp_farm_claim_rewards(
            original_owner.clone(),
            lp_farm_token_id,
            attributes.lp_farm_token_nonce,
            attributes.lp_farm_token_amount,
        );
        let staking_farm_claim_rewards_result = self.staking_farm_claim_rewards(
            original_owner.clone(),
            staking_farm_token_id,
            attributes.staking_farm_token_nonce,
            attributes.staking_farm_token_amount,
            new_staking_farm_value,
        );

        let new_lp_farm_tokens = lp_farm_claim_rewards_result.new_lp_farm_tokens;
        let new_staking_farm_tokens = staking_farm_claim_rewards_result.new_staking_farm_tokens;
        let new_attributes = DualYieldTokenAttributes {
            lp_farm_token_nonce: new_lp_farm_tokens.token_nonce,
            lp_farm_token_amount: new_lp_farm_tokens.amount,
            staking_farm_token_nonce: new_staking_farm_tokens.token_nonce,
            staking_farm_token_amount: new_staking_farm_tokens.amount,
        };

        let lp_farm_rewards = lp_farm_claim_rewards_result.lp_farm_rewards;
        let staking_farm_rewards = staking_farm_claim_rewards_result.staking_farm_rewards;
        let new_dual_yield_attributes = new_attributes;

        let new_dual_yield_tokens =
            self.create_dual_yield_tokens(&dual_yield_token_mapper, &new_dual_yield_attributes);
        let claim_result = ClaimDualYieldResult {
            lp_farm_rewards,
            staking_farm_rewards,
            new_dual_yield_tokens,
        };

        self.send_payment_non_zero(&original_owner, &claim_result.lp_farm_rewards);
        self.send_payment_non_zero(&original_owner, &claim_result.staking_farm_rewards);
        self.send_payment_non_zero(&caller, &claim_result.new_dual_yield_tokens);

        dual_yield_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        claim_result
    }

    fn check_stake_farm_payments(
        &self,
        original_owner: &ManagedAddress,
        first_payment: &EsdtTokenPayment,
        additional_payments: &ManagedVec<EsdtTokenPayment>,
    ) {
        let dual_yield_token_mapper = self.dual_yield_token();
        let dual_yield_token_id = dual_yield_token_mapper.get_token_id();
        let lp_farm_token_id = self.lp_farm_token_id().get();

        require!(
            first_payment.token_identifier == lp_farm_token_id,
            "Invalid first payment"
        );
        require!(
            &self.get_farm_position_original_owner(first_payment.token_nonce) == original_owner,
            "Provided address is not the same as the original owner"
        );

        for payment in additional_payments.into_iter() {
            if payment.token_identifier != dual_yield_token_id {
                sc_panic!("Wrong additional payments");
            }

            let attributes: DualYieldTokenAttributes<Self::Api> =
                dual_yield_token_mapper.get_token_attributes(payment.token_nonce);
            require!(
                &self.get_farm_position_original_owner(attributes.lp_farm_token_nonce)
                    == original_owner,
                "Provided address is not the same as the original owner"
            );
        }
    }

    fn get_farm_position_original_owner(&self, farm_token_nonce: u64) -> ManagedAddress {
        let lp_farm_token_id = self.lp_farm_token_id().get();
        let attributes = self
            .blockchain()
            .get_token_attributes::<FarmTokenAttributes<Self::Api>>(
                &lp_farm_token_id,
                farm_token_nonce,
            );

        attributes.original_owner
    }

    fn require_user_whitelisted(&self, user: &ManagedAddress, authorized_address: &ManagedAddress) {
        let permissions_hub_address = self.permissions_hub_address().get();
        let is_whitelisted: bool = self
            .permissions_hub_proxy(permissions_hub_address)
            .is_whitelisted(user, authorized_address)
            .execute_on_dest_context();

        require!(is_whitelisted, "Caller is not whitelisted by the user");
    }

    #[only_owner]
    #[endpoint(setPermissionsHubAddress)]
    fn set_permissions_hub_address(&self, address: ManagedAddress) {
        self.permissions_hub_address().set(&address);
    }

    #[proxy]
    fn permissions_hub_proxy(
        &self,
        sc_address: ManagedAddress,
    ) -> permissions_hub::Proxy<Self::Api>;

    #[storage_mapper("permissionsHubAddress")]
    fn permissions_hub_address(&self) -> SingleValueMapper<ManagedAddress>;
}
