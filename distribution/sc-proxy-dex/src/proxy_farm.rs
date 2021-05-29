#![allow(clippy::too_many_arguments)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;
use distrib_common::*;

use super::proxy_common;
use super::proxy_pair;
pub use dex_common::*;

const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";

type EnterFarmResultType<BigUint> = GenericEsdtAmountPair<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<GenericEsdtAmountPair<BigUint>, GenericEsdtAmountPair<BigUint>>;
type ExitFarmResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, GenericEsdtAmountPair<BigUint>>;

#[elrond_wasm_derive::module]
pub trait ProxyFarmModule: proxy_common::ProxyCommonModule + proxy_pair::ProxyPairModule {
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

        self.reset_received_funds_on_current_tx();
        let farm_result =
            self.actual_enter_farm(&farm_address, &farming_token_id, &amount, with_lock_rewards);
        let farm_token_id = farm_result.token_id;
        let farm_token_nonce = farm_result.token_nonce;
        let farm_token_total_amount = farm_result.amount;
        require!(
            farm_token_total_amount > 0,
            "Farm token amount received should be greater than 0"
        );
        self.validate_received_funds_chunk(
            [(&farm_token_id, farm_token_nonce, &farm_token_total_amount)].to_vec(),
        )?;

        let attributes = WrappedFarmTokenAttributes {
            farm_token_id,
            farm_token_nonce,
            farming_token_id: token_id,
            farming_token_nonce: token_nonce,
        };
        let caller = self.blockchain().get_caller();
        self.create_and_send_wrapped_farm_tokens(&attributes, &farm_token_total_amount, &caller);

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
        self.require_is_intermediated_farm(&farm_address)?;
        self.require_wrapped_farm_token_id_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.wrapped_farm_token_id().get(),
            "Should only be used with wrapped farm tokens"
        );

        let wrapped_farm_token_attrs = self.get_attributes(&token_id, token_nonce)?;
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id;
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        self.reset_received_funds_on_current_tx();
        let farm_result = self
            .actual_exit_farm(&farm_address, &farm_token_id, farm_token_nonce, &amount)
            .into_tuple();
        let farming_token_returned = farm_result.0;
        let reward_token_returned = farm_result.1;
        self.validate_received_funds_chunk(
            [
                (
                    &farming_token_returned.token_id,
                    0,
                    &farming_token_returned.amount,
                ),
                (
                    &reward_token_returned.token_id,
                    reward_token_returned.token_nonce,
                    &reward_token_returned.amount,
                ),
            ]
            .to_vec(),
        )?;

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
        self.send().esdt_nft_burn(&token_id, token_nonce, &amount);
        if farming_token_returned.token_id == self.asset_token_id().get() {
            self.send().esdt_local_burn(
                &farming_token_returned.token_id,
                &farming_token_returned.amount,
            );
        }

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
        require!(amount != 0, "Payment amount cannot be zero");
        require!(
            token_id == self.wrapped_farm_token_id().get(),
            "Should only be used with wrapped farm tokens"
        );

        // Read info about wrapped farm token and then burn it.
        let wrapped_farm_token_attrs = self.get_attributes(&token_id, token_nonce)?;
        let farm_token_id = wrapped_farm_token_attrs.farm_token_id;
        let farm_token_nonce = wrapped_farm_token_attrs.farm_token_nonce;

        self.reset_received_funds_on_current_tx();
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
        self.validate_received_funds_chunk(
            [
                (
                    &new_farm_token_id,
                    new_farm_token_nonce,
                    &new_farm_token_total_amount,
                ),
                (
                    &reward_token_returned.token_id,
                    reward_token_returned.token_nonce,
                    &reward_token_returned.amount,
                ),
            ]
            .to_vec(),
        )?;

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
            farming_token_id: wrapped_farm_token_attrs.farming_token_id,
            farming_token_nonce: wrapped_farm_token_attrs.farming_token_nonce,
        };
        self.create_and_send_wrapped_farm_tokens(
            &new_wrapped_farm_token_attributes,
            &new_farm_token_total_amount,
            &caller,
        );
        self.send().esdt_nft_burn(&token_id, token_nonce, &amount);

        Ok(())
    }

    fn get_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> SCResult<WrappedFarmTokenAttributes> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        let attributes = token_info.decode_attributes::<WrappedFarmTokenAttributes>();
        match attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn create_and_send_wrapped_farm_tokens(
        &self,
        attributes: &WrappedFarmTokenAttributes,
        amount: &Self::BigUint,
        address: &Address,
    ) {
        let wrapped_farm_token_id = self.wrapped_farm_token_id().get();
        self.create_wrapped_farm_tokens(&wrapped_farm_token_id, attributes, amount);
        let nonce = self.wrapped_farm_token_nonce().get();
        self.send()
            .direct_nft(address, &wrapped_farm_token_id, nonce, amount, &[]);
    }

    fn create_wrapped_farm_tokens(
        &self,
        token_id: &TokenIdentifier,
        attributes: &WrappedFarmTokenAttributes,
        amount: &Self::BigUint,
    ) {
        self.send().esdt_nft_create::<WrappedFarmTokenAttributes>(
            token_id,
            amount,
            &BoxedBytes::empty(),
            &Self::BigUint::zero(),
            &BoxedBytes::empty(),
            &attributes,
            &[BoxedBytes::empty()],
        );
        self.increase_wrapped_farm_token_nonce();
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
            self.send().esdt_local_mint(&asset_token_id, &amount);
        }
        if with_locked_rewards {
            self.farm_contract_proxy(farm_address.clone())
                .enterFarmAndLockRewards(
                    farming_token_id.clone(),
                    amount.clone(),
                    OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
                )
                .execute_on_dest_context_custom_range(|_, after| (after - 1, after))
        } else {
            self.farm_contract_proxy(farm_address.clone())
                .enterFarm(
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
            .exitFarm(
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
            .claimRewards(
                farm_token_id.clone(),
                amount.clone(),
                farm_token_nonce,
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context_custom_range(|_, after| (after - 2, after))
    }

    fn increase_wrapped_farm_token_nonce(&self) -> Nonce {
        let new_nonce = self.wrapped_farm_token_nonce().get() + 1;
        self.wrapped_farm_token_nonce().set(&new_nonce);
        new_nonce
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

    #[view(getIntermediatedFarms)]
    #[storage_mapper("intermediated_farms")]
    fn intermediated_farms(&self) -> SetMapper<Self::Storage, Address>;

    #[view(getWrappedFarmTokenId)]
    #[storage_mapper("wrapped_farm_token_id")]
    fn wrapped_farm_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("wrapped_farm_token_nonce")]
    fn wrapped_farm_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;
}
