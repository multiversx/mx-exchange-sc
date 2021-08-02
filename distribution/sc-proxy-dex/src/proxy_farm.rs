#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{
    FftTokenAmountPair, GenericTokenAmountPair, Nonce, WrappedFarmTokenAttributes,
};

use super::events;
use super::proxy_common;
use super::proxy_pair;
use super::wrapped_farm_token_merge;
use super::wrapped_lp_token_merge;

use proxy_common::ACCEPT_PAY_FUNC_NAME;

type EnterFarmResultType<BigUint> = GenericTokenAmountPair<BigUint>;
type CompoundRewardsResultType<BigUint> = GenericTokenAmountPair<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<GenericTokenAmountPair<BigUint>, GenericTokenAmountPair<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, GenericTokenAmountPair<BigUint>>;

#[derive(Clone)]
pub struct WrappedFarmToken<BigUint: BigUintApi> {
    pub token_amount: GenericTokenAmountPair<BigUint>,
    pub attributes: WrappedFarmTokenAttributes<BigUint>,
}

#[elrond_wasm_derive::module]
pub trait ProxyFarmModule:
    proxy_common::ProxyCommonModule
    + proxy_pair::ProxyPairModule
    + token_supply::TokenSupplyModule
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + nft_deposit::NftDepositModule
    + wrapped_farm_token_merge::WrappedFarmTokenMerge
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + events::EventsModule
{
    #[proxy]
    fn farm_contract_proxy(&self, to: Address) -> elrond_dex_farm::Proxy<Self::SendApi>;

    #[endpoint(addFarmToIntermediate)]
    fn add_farm_to_intermediate(&self, farm_address: Address) -> SCResult<()> {
        self.require_permissions()?;
        self.intermediated_farms().insert(farm_address);
        Ok(())
    }

    #[endpoint(removeIntermediatedFarm)]
    fn remove_intermediated_farm(&self, farm_address: Address) -> SCResult<()> {
        self.require_permissions()?;
        self.require_is_intermediated_farm(&farm_address)?;
        self.intermediated_farms().remove(&farm_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(enterFarmProxy)]
    fn enter_farm_proxy_endpoint(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] nonce: Nonce,
        farm_address: Address,
    ) -> SCResult<()> {
        self.enter_farm_proxy(token_id, nonce, amount, farm_address, false)
    }

    #[payable("*")]
    #[endpoint(enterFarmAndLockRewardsProxy)]
    fn enter_farm_and_lock_rewards_proxy_endpoint(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] nonce: Nonce,
        farm_address: Address,
    ) -> SCResult<()> {
        self.enter_farm_proxy(token_id, nonce, amount, farm_address, true)
    }

    fn enter_farm_proxy(
        &self,
        token_id: TokenIdentifier,
        token_nonce: Nonce,
        amount: Self::BigUint,
        farm_address: Address,
        with_lock_rewards: bool,
    ) -> SCResult<()> {
        self.require_is_intermediated_farm(&farm_address)?;
        self.require_wrapped_farm_token_id_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        self.require_deposit_empty_or_tokens_are_wrapped_farm_tokens()?;
        require!(amount != 0, "Payment amount cannot be zero");

        let farming_token_id: TokenIdentifier;
        if token_id == self.wrapped_lp_token_id().get() {
            let wrapped_lp_token_attrs =
                self.get_wrapped_lp_token_attributes(&token_id, token_nonce)?;
            farming_token_id = wrapped_lp_token_attrs.lp_token_id;
        } else if token_id == self.locked_asset_token_id().get() {
            let asset_token_id = self.asset_token_id().get();
            farming_token_id = asset_token_id;
        } else {
            return sc_error!("Unknown input Token");
        }

        let farm_result =
            self.actual_enter_farm(&farm_address, &farming_token_id, &amount, with_lock_rewards);
        let farm_token_id = farm_result.token_id;
        let farm_token_nonce = farm_result.token_nonce;
        let farm_token_total_amount = farm_result.amount;
        require!(
            farm_token_total_amount > 0,
            "Farm token amount received should be greater than 0"
        );

        let attributes = WrappedFarmTokenAttributes {
            farm_token_id,
            farm_token_nonce,
            farm_token_amount: farm_token_total_amount.clone(),
            farming_token_id: token_id.clone(),
            farming_token_nonce: token_nonce,
            farming_token_amount: amount.clone(),
        };
        let caller = self.blockchain().get_caller();
        let (new_wrapped_farm_token, created_with_merge) = self
            .create_wrapped_farm_tokens_by_merging_and_send(
                &attributes,
                &farm_token_total_amount,
                &farm_address,
                &caller,
            )?;

        let farming_token_amount = GenericTokenAmountPair {
            token_id,
            token_nonce,
            amount,
        };
        self.emit_enter_farm_proxy_event(
            caller,
            farm_address,
            farming_token_amount,
            new_wrapped_farm_token.token_amount,
            new_wrapped_farm_token.attributes,
            created_with_merge,
        );
        Ok(())
    }

    #[payable("*")]
    #[endpoint(exitFarmProxy)]
    fn exit_farm_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
        farm_address: &Address,
    ) -> SCResult<()> {
        self.require_is_intermediated_farm(farm_address)?;
        self.require_wrapped_farm_token_id_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.wrapped_farm_token_id().get(),
            "Should only be used with wrapped farm tokens"
        );

        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&token_id, token_nonce)?;
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        let farm_result = self
            .actual_exit_farm(farm_address, &farm_token_id, farm_token_nonce, &amount)
            .into_tuple();
        let farming_token_returned = farm_result.0;
        let reward_token_returned = farm_result.1;

        let caller = self.blockchain().get_caller();
        self.send().direct_nft(
            &caller,
            &wrapped_farm_token_attrs.farming_token_id,
            wrapped_farm_token_attrs.farming_token_nonce,
            &farming_token_returned.amount,
            &[],
        );

        self.direct_generic_safe(
            &caller,
            &reward_token_returned.token_id,
            reward_token_returned.token_nonce,
            &reward_token_returned.amount,
        );
        self.nft_burn_tokens(&token_id, token_nonce, &amount);

        if farming_token_returned.token_id == self.asset_token_id().get() {
            self.burn_tokens(
                &farming_token_returned.token_id,
                &farming_token_returned.amount,
            );
        }

        let farming_tokens = GenericTokenAmountPair {
            token_id: wrapped_farm_token_attrs.farming_token_id.clone(),
            token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
            amount: farming_token_returned.amount,
        };
        let wrapped_farm_token_amount = GenericTokenAmountPair {
            token_id,
            token_nonce,
            amount,
        };
        self.emit_exit_farm_proxy_event(
            caller,
            farm_address.clone(),
            wrapped_farm_token_amount,
            wrapped_farm_token_attrs,
            farming_tokens,
            reward_token_returned,
        );
        Ok(())
    }

    #[payable("*")]
    #[endpoint(claimRewardsProxy)]
    fn claim_rewards_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
        farm_address: Address,
    ) -> SCResult<()> {
        self.require_is_intermediated_farm(&farm_address)?;
        self.require_wrapped_farm_token_id_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        self.require_deposit_empty_or_tokens_are_wrapped_farm_tokens()?;
        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.wrapped_farm_token_id().get(),
            "Should only be used with wrapped farm tokens"
        );

        // Read info about wrapped farm token and then burn it.
        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&token_id, token_nonce)?;
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        let result = self
            .actual_claim_rewards(&farm_address, &farm_token_id, farm_token_nonce, &amount)
            .into_tuple();
        let new_farm_token = result.0;
        let reward_token_returned = result.1;
        let new_farm_token_id = new_farm_token.token_id;
        let new_farm_token_nonce = new_farm_token.token_nonce;
        let new_farm_token_total_amount = new_farm_token.amount;
        require!(
            new_farm_token_total_amount > 0,
            "Farm token amount received should be greater than 0"
        );

        // Send the reward to the caller.
        let caller = self.blockchain().get_caller();
        self.direct_generic_safe(
            &caller,
            &reward_token_returned.token_id,
            reward_token_returned.token_nonce,
            &reward_token_returned.amount,
        );

        // Create new Wrapped tokens and send them.
        let new_wrapped_farm_token_attributes = WrappedFarmTokenAttributes {
            farm_token_id: new_farm_token_id,
            farm_token_nonce: new_farm_token_nonce,
            farm_token_amount: new_farm_token_total_amount.clone(),
            farming_token_id: wrapped_farm_token_attrs.farming_token_id.clone(),
            farming_token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
            farming_token_amount: self.rule_of_three_non_zero_result(
                &amount,
                &wrapped_farm_token_attrs.farm_token_amount,
                &wrapped_farm_token_attrs.farming_token_amount,
            )?,
        };
        let (new_wrapped_farm, created_with_merge) = self
            .create_wrapped_farm_tokens_by_merging_and_send(
                &new_wrapped_farm_token_attributes,
                &new_farm_token_total_amount,
                &farm_address,
                &caller,
            )?;
        self.nft_burn_tokens(&token_id, token_nonce, &amount);

        let old_wrapped_farm_token_amount = GenericTokenAmountPair {
            token_id,
            token_nonce,
            amount,
        };
        self.emit_claim_rewards_farm_proxy_event(
            caller,
            farm_address,
            old_wrapped_farm_token_amount,
            new_wrapped_farm.token_amount,
            reward_token_returned,
            wrapped_farm_token_attrs,
            new_wrapped_farm.attributes,
            created_with_merge,
        );
        Ok(())
    }

    #[payable("*")]
    #[endpoint(compoundRewardsProxy)]
    fn compound_rewards_proxy(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] payment_token_nonce: Nonce,
        #[payment_amount] payment_amount: Self::BigUint,
        farm_address: Address,
    ) -> SCResult<()> {
        self.require_is_intermediated_farm(&farm_address)?;
        self.require_wrapped_farm_token_id_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        self.require_deposit_empty_or_tokens_are_wrapped_farm_tokens()?;
        require!(payment_amount != 0, "Payment amount cannot be zero");
        let wrapped_farm_token = self.wrapped_farm_token_id().get();
        require!(
            payment_token_id == wrapped_farm_token,
            "Should only be used with wrapped farm tokens"
        );

        let wrapped_farm_token_attrs =
            self.get_wrapped_farm_token_attributes(&payment_token_id, payment_token_nonce)?;
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id.clone();
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;
        let farm_amount = payment_amount.clone();

        let result = self.actual_compound_rewards(
            &farm_address,
            &farm_token_id,
            farm_token_nonce,
            &farm_amount,
        );

        let new_farm_token = result;
        let new_farm_token_id = new_farm_token.token_id;
        let new_farm_token_nonce = new_farm_token.token_nonce;
        let new_farm_token_amount = new_farm_token.amount;
        require!(
            new_farm_token_amount > 0,
            "Farm token amount received should be greater than 0"
        );

        let new_wrapped_farm_token_attributes = WrappedFarmTokenAttributes {
            farm_token_id: new_farm_token_id,
            farm_token_nonce: new_farm_token_nonce,
            farm_token_amount: new_farm_token_amount.clone(),
            farming_token_id: wrapped_farm_token_attrs.farming_token_id.clone(),
            farming_token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
            farming_token_amount: self.rule_of_three_non_zero_result(
                &payment_amount,
                &wrapped_farm_token_attrs.farm_token_amount,
                &wrapped_farm_token_attrs.farming_token_amount,
            )?,
        };
        let caller = self.blockchain().get_caller();
        let (new_wrapped_farm, created_with_merge) = self
            .create_wrapped_farm_tokens_by_merging_and_send(
                &new_wrapped_farm_token_attributes,
                &new_farm_token_amount,
                &farm_address,
                &caller,
            )?;
        self.nft_burn_tokens(&payment_token_id, payment_token_nonce, &payment_amount);

        let old_wrapped_farm_token_amount = GenericTokenAmountPair {
            token_id: payment_token_id,
            token_nonce: payment_token_nonce,
            amount: payment_amount,
        };
        self.emit_compound_rewards_farm_proxy_event(
            caller,
            farm_address,
            old_wrapped_farm_token_amount,
            new_wrapped_farm.token_amount,
            wrapped_farm_token_attrs,
            new_wrapped_farm.attributes,
            created_with_merge,
        );
        Ok(())
    }

    fn create_wrapped_farm_tokens_by_merging_and_send(
        &self,
        attributes: &WrappedFarmTokenAttributes<Self::BigUint>,
        amount: &Self::BigUint,
        farm_address: &Address,
        caller: &Address,
    ) -> SCResult<(WrappedFarmToken<Self::BigUint>, bool)> {
        let wrapped_farm_token_id = self.wrapped_farm_token_id().get();
        self.merge_wrapped_farm_tokens_and_send(
            caller,
            farm_address,
            Option::Some(WrappedFarmToken {
                token_amount: GenericTokenAmountPair {
                    token_id: wrapped_farm_token_id,
                    token_nonce: 0,
                    amount: amount.clone(),
                },
                attributes: attributes.clone(),
            }),
            OptionalArg::None,
        )
    }

    fn actual_enter_farm(
        &self,
        farm_address: &Address,
        farming_token_id: &TokenIdentifier,
        amount: &Self::BigUint,
        with_locked_rewards: bool,
    ) -> EnterFarmResultType<Self::BigUint> {
        let asset_token_id = self.asset_token_id().get();
        if farming_token_id == &asset_token_id {
            self.mint_tokens(&asset_token_id, amount);
        }
        if with_locked_rewards {
            self.farm_contract_proxy(farm_address.clone())
                .enter_farm_and_lock_rewards(
                    farming_token_id.clone(),
                    amount.clone(),
                    OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
                )
                .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
        } else {
            self.farm_contract_proxy(farm_address.clone())
                .enter_farm(
                    farming_token_id.clone(),
                    amount.clone(),
                    OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
                )
                .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
        }
    }

    fn actual_exit_farm(
        &self,
        farm_address: &Address,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &Self::BigUint,
    ) -> ExitFarmResultType<Self::BigUint> {
        self.farm_contract_proxy(farm_address.clone())
            .exit_farm(
                farm_token_id.clone(),
                amount.clone(),
                farm_token_nonce,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
    }

    fn actual_claim_rewards(
        &self,
        farm_address: &Address,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &Self::BigUint,
    ) -> ClaimRewardsResultType<Self::BigUint> {
        self.farm_contract_proxy(farm_address.clone())
            .claim_rewards(
                farm_token_id.clone(),
                amount.clone(),
                farm_token_nonce,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
    }

    fn actual_compound_rewards(
        &self,
        farm_address: &Address,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &Self::BigUint,
    ) -> CompoundRewardsResultType<Self::BigUint> {
        self.farm_contract_proxy(farm_address.clone())
            .compound_rewards(
                farm_token_id.clone(),
                amount.clone(),
                farm_token_nonce,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
    }

    fn require_is_intermediated_farm(&self, address: &Address) -> SCResult<()> {
        require!(
            self.intermediated_farms().contains(address),
            "Not an intermediated farm"
        );
        Ok(())
    }

    fn require_wrapped_farm_token_id_not_empty(&self) -> SCResult<()> {
        require!(!self.wrapped_farm_token_id().is_empty(), "Empty token id");
        Ok(())
    }
}
