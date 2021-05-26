#![allow(clippy::too_many_arguments)]
#![allow(clippy::comparison_chain)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

const ACCEPT_PAY_FUNC_NAME: &[u8] = b"acceptPay";

use dex_common::*;
use distrib_common::*;

use super::proxy_common;

type AddLiquidityResultType<BigUint> = MultiResult3<
    FftTokenAmountPair<BigUint>,
    FftTokenAmountPair<BigUint>,
    FftTokenAmountPair<BigUint>,
>;

type RemoveLiquidityResultType<BigUint> =
    MultiResult2<FftTokenAmountPair<BigUint>, FftTokenAmountPair<BigUint>>;

#[derive(TopEncode, TopDecode, PartialEq, Clone, Copy, TypeAbi)]
pub struct ProxyPairParams {
    pub add_liquidity_gas_limit: u64,
    pub accept_esdt_payment_gas_limit: u64,
    pub ask_for_lp_token_gas_limit: u64,
    pub remove_liquidity_gas_limit: u64,
    pub burn_tokens_gas_limit: u64,
    pub mint_tokens_gas_limit: u64,
}

#[elrond_wasm_derive::module]
pub trait ProxyPairModule: proxy_common::ProxyCommonModule {
    #[proxy]
    fn pair_contract_proxy(&self, to: Address) -> elrond_dex_pair::Proxy<Self::SendApi>;

    fn init_proxy_pair(&self, proxy_params: ProxyPairParams) {
        self.proxy_pair_params().set(&proxy_params);
    }

    #[endpoint(setProxyPairParams)]
    fn set_proxy_pair_params(&self, proxy_params: ProxyPairParams) -> SCResult<()> {
        self.require_permissions()?;
        self.proxy_pair_params().set(&proxy_params);
        Ok(())
    }

    #[endpoint(addPairToIntermediate)]
    fn add_pair_to_intermediate(&self, pair_address: Address) -> SCResult<()> {
        self.require_permissions()?;
        self.intermediated_pairs().insert(pair_address);
        Ok(())
    }

    #[endpoint(removeIntermediatedPair)]
    fn remove_intermediated_pair(&self, pair_address: Address) -> SCResult<()> {
        self.require_permissions()?;
        self.require_is_intermediated_pair(&pair_address)?;
        self.intermediated_pairs().remove(&pair_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(acceptEsdtPaymentProxy)]
    fn accept_esdt_payment_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment] amount: Self::BigUint,
        pair_address: Address,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;

        let token_nonce = self.call_value().esdt_token_nonce();
        require!(amount != 0, "Payment amount cannot be zero");

        let caller = self.blockchain().get_caller();
        self.increase_temporary_funds_amount(&caller, &token_id, token_nonce, &amount);
        Ok(())
    }

    #[endpoint(reclaimTemporaryFundsProxy)]
    fn reclaim_temporary_funds_proxy(
        &self,
        first_token_id: TokenIdentifier,
        first_token_nonce: Nonce,
        second_token_id: TokenIdentifier,
        second_token_nonce: Nonce,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        self.send_temporary_funds_back(&caller, &first_token_id, first_token_nonce);
        self.send_temporary_funds_back(&caller, &second_token_id, second_token_nonce);
        Ok(())
    }

    #[endpoint(addLiquidityProxy)]
    fn add_liquidity_proxy(
        &self,
        pair_address: Address,
        first_token_id: TokenIdentifier,
        first_token_nonce: Nonce,
        first_token_amount_desired: Self::BigUint,
        first_token_amount_min: Self::BigUint,
        second_token_id: TokenIdentifier,
        second_token_nonce: Nonce,
        second_token_amount_desired: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.require_proxy_pair_params_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        let proxy_params = self.proxy_pair_params().get();

        let caller = self.blockchain().get_caller();
        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            (first_token_nonce == 0 && second_token_nonce != 0)
                || (first_token_nonce != 0 && second_token_nonce == 0),
            "This endpoint accepts one Fungible and one SemiFungible"
        );
        require!(
            first_token_amount_desired > 0 && second_token_amount_desired > 0,
            "Cannot add zero amount"
        );
        let locked_asset_token_id = if self.accepted_locked_assets().contains(&first_token_id) {
            first_token_id.clone()
        } else if self.accepted_locked_assets().contains(&second_token_id) {
            second_token_id.clone()
        } else {
            return sc_error!("One token should be an accepted locked asset token");
        };
        let first_token_amount_temporary = self
            .temporary_funds(&caller, &first_token_id, first_token_nonce)
            .get();
        require!(
            first_token_amount_temporary >= first_token_amount_desired,
            "Not enough first temporary funds"
        );
        let second_token_amount_temporary = self
            .temporary_funds(&caller, &second_token_id, second_token_nonce)
            .get();
        require!(
            second_token_amount_temporary >= second_token_amount_desired,
            "Not enough second temporary funds"
        );

        // Actual 2x acceptEsdtPayment
        self.forward_to_pair(
            &pair_address,
            &first_token_id,
            first_token_nonce,
            &first_token_amount_desired,
            &proxy_params,
        );
        self.forward_to_pair(
            &pair_address,
            &second_token_id,
            second_token_nonce,
            &second_token_amount_desired,
            &proxy_params,
        );

        // Actual adding of liquidity
        self.reset_received_funds_on_current_tx();
        let result = self.actual_add_liquidity(
            &pair_address,
            &first_token_amount_desired,
            &first_token_amount_min,
            &second_token_amount_desired,
            &second_token_amount_min,
            &proxy_params,
        );

        let result_tuple = result.0;
        let lp_received = result_tuple.0;
        let first_token_used = result_tuple.1;
        let second_token_used = result_tuple.2;
        require!(
            lp_received.amount > 0,
            "LP token amount should be greater than 0"
        );
        require!(
            first_token_used.token_id == first_token_id
                || second_token_used.token_id == second_token_id,
            "Bad token order"
        );

        let mut desired_received_funds_size = 1;
        self.validate_received_funds_on_current_tx(&lp_received.token_id, 0, &lp_received.amount)?;
        if first_token_amount_desired > first_token_used.amount {
            desired_received_funds_size += 1;
            self.validate_received_funds_on_current_tx(
                &first_token_used.token_id,
                0,
                &(&first_token_amount_desired - &first_token_used.amount),
            )?;
        }

        if second_token_amount_desired > second_token_used.amount {
            desired_received_funds_size += 1;
            self.validate_received_funds_on_current_tx(
                &second_token_used.token_id,
                0,
                &(&second_token_amount_desired - &second_token_used.amount),
            )?;
        }
        self.validate_received_funds_on_current_tx_size(desired_received_funds_size)?;

        //Recalculate temporary funds and burn unused
        let locked_asset_token_nonce: Nonce;
        let consumed_locked_tokens: Self::BigUint;
        let asset_token_id = self.asset_token_id().get();
        if first_token_used.token_id == asset_token_id {
            consumed_locked_tokens = first_token_used.amount;
            let unused_minted_assets = first_token_amount_desired - consumed_locked_tokens.clone();
            self.send().burn_tokens(
                &asset_token_id,
                0,
                &unused_minted_assets,
                proxy_params.burn_tokens_gas_limit,
            );
            locked_asset_token_nonce = first_token_nonce;

            self.decrease_temporary_funds_amount(
                &caller,
                &first_token_id,
                first_token_nonce,
                &consumed_locked_tokens,
            );
            self.decrease_temporary_funds_amount(
                &caller,
                &second_token_used.token_id,
                second_token_nonce,
                &second_token_used.amount,
            );
        } else if second_token_used.token_id == asset_token_id {
            consumed_locked_tokens = second_token_used.amount;
            let unused_minted_assets = second_token_amount_desired - consumed_locked_tokens.clone();
            self.send().burn_tokens(
                &asset_token_id,
                0,
                &unused_minted_assets,
                proxy_params.burn_tokens_gas_limit,
            );
            locked_asset_token_nonce = second_token_nonce;

            self.decrease_temporary_funds_amount(
                &caller,
                &first_token_used.token_id,
                first_token_nonce,
                &first_token_used.amount,
            );
            self.decrease_temporary_funds_amount(
                &caller,
                &second_token_id,
                second_token_nonce,
                &consumed_locked_tokens,
            );
        } else {
            return sc_error!("Add liquidity did not return asset token id");
        }

        self.send_temporary_funds_back(&caller, &first_token_id, first_token_nonce);
        self.send_temporary_funds_back(&caller, &second_token_id, second_token_nonce);
        self.create_and_send(
            &lp_received.token_id,
            &lp_received.amount,
            &locked_asset_token_id,
            &consumed_locked_tokens,
            locked_asset_token_nonce,
            &caller,
            &proxy_params,
        );

        Ok(())
    }

    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment] amount: Self::BigUint,
        pair_address: Address,
        first_token_amount_min: Self::BigUint,
        second_token_amount_min: Self::BigUint,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.require_proxy_pair_params_not_empty()?;
        self.require_wrapped_lp_token_id_not_empty()?;
        let proxy_params = self.proxy_pair_params().get();

        let token_nonce = self.call_value().esdt_token_nonce();
        require!(token_nonce != 0, "Can only be called with an SFT");
        require!(amount != 0, "Payment amount cannot be zero");

        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
        require!(token_id == wrapped_lp_token_id, "Wrong input token");

        let caller = self.blockchain().get_caller();
        let lp_token_id = self.ask_for_lp_token_id(&pair_address, &proxy_params);
        let attributes = self.get_wrapped_lp_token_attributes(&token_id, token_nonce)?;
        require!(lp_token_id == attributes.lp_token_id, "Bad input address");

        let locked_asset_token_id = attributes.locked_assets_token_id;
        let asset_token_id = self.asset_token_id().get();

        self.reset_received_funds_on_current_tx();
        let tokens_for_position = self
            .actual_remove_liquidity(
                &pair_address,
                &lp_token_id,
                &amount,
                &first_token_amount_min,
                &second_token_amount_min,
                &proxy_params,
            )
            .into_tuple();

        self.validate_received_funds_on_current_tx_size(2)?;
        self.validate_received_funds_on_current_tx(
            &tokens_for_position.0.token_id,
            0,
            &tokens_for_position.0.amount,
        )?;
        self.validate_received_funds_on_current_tx(
            &tokens_for_position.1.token_id,
            0,
            &tokens_for_position.1.amount,
        )?;

        let fungible_token_id: TokenIdentifier;
        let fungible_token_amount: Self::BigUint;
        let assets_received: Self::BigUint;
        let locked_assets_invested =
            amount.clone() * attributes.locked_assets_invested / attributes.lp_token_total_amount;
        require!(
            locked_assets_invested > 0,
            "Not enough wrapped lp token provided"
        );
        if tokens_for_position.0.token_id == asset_token_id {
            assets_received = tokens_for_position.0.amount;
            fungible_token_id = tokens_for_position.1.token_id;
            fungible_token_amount = tokens_for_position.1.amount;
        } else if tokens_for_position.1.token_id == asset_token_id {
            assets_received = tokens_for_position.1.amount;
            fungible_token_id = tokens_for_position.0.token_id;
            fungible_token_amount = tokens_for_position.0.amount;
        } else {
            return sc_error!("Bad tokens received from pair SC");
        }

        //Send back the tokens removed from pair sc.
        self.send()
            .transfer_tokens(&fungible_token_id, 0, &fungible_token_amount, &caller);
        let locked_assets_to_send =
            core::cmp::min(assets_received.clone(), locked_assets_invested.clone());
        self.send().transfer_tokens(
            &locked_asset_token_id,
            attributes.locked_assets_nonce,
            &locked_assets_to_send,
            &caller,
        );

        //Do cleanup
        if assets_received > locked_assets_invested {
            let difference = assets_received - locked_assets_invested.clone();
            self.send()
                .transfer_tokens(&asset_token_id, 0, &difference, &caller);
            self.send().burn_tokens(
                &asset_token_id,
                0,
                &locked_assets_invested,
                proxy_params.burn_tokens_gas_limit,
            );
        } else if assets_received < locked_assets_invested {
            let difference = locked_assets_invested - assets_received.clone();
            self.send().burn_tokens(
                &locked_asset_token_id,
                attributes.locked_assets_nonce,
                &difference,
                proxy_params.burn_tokens_gas_limit,
            );
            self.send().burn_tokens(
                &asset_token_id,
                0,
                &assets_received,
                proxy_params.burn_tokens_gas_limit,
            );
        } else {
            self.send().burn_tokens(
                &asset_token_id,
                0,
                &assets_received,
                proxy_params.burn_tokens_gas_limit,
            );
        }

        self.send().burn_tokens(
            &wrapped_lp_token_id,
            token_nonce,
            &amount,
            proxy_params.burn_tokens_gas_limit,
        );
        Ok(())
    }

    fn actual_add_liquidity(
        &self,
        pair_address: &Address,
        first_token_amount_desired: &Self::BigUint,
        first_token_amount_min: &Self::BigUint,
        second_token_amount_desired: &Self::BigUint,
        second_token_amount_min: &Self::BigUint,
        proxy_params: &ProxyPairParams,
    ) -> AddLiquidityResultType<Self::BigUint> {
        self.pair_contract_proxy(pair_address.clone())
            .addLiquidity(
                first_token_amount_desired.clone(),
                second_token_amount_desired.clone(),
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context(proxy_params.add_liquidity_gas_limit)
    }

    fn actual_remove_liquidity(
        &self,
        pair_address: &Address,
        lp_token_id: &TokenIdentifier,
        liquidity: &Self::BigUint,
        first_token_amount_min: &Self::BigUint,
        second_token_amount_min: &Self::BigUint,
        proxy_params: &ProxyPairParams,
    ) -> RemoveLiquidityResultType<Self::BigUint> {
        self.pair_contract_proxy(pair_address.clone())
            .removeLiquidity(
                lp_token_id.clone(),
                liquidity.clone(),
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
                OptionalArg::Some(BoxedBytes::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context(proxy_params.remove_liquidity_gas_limit)
    }

    fn ask_for_lp_token_id(
        &self,
        pair_address: &Address,
        proxy_params: &ProxyPairParams,
    ) -> TokenIdentifier {
        self.pair_contract_proxy(pair_address.clone())
            .getLpTokenIdentifier()
            .execute_on_dest_context(proxy_params.ask_for_lp_token_gas_limit)
    }

    fn get_wrapped_lp_token_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> SCResult<WrappedLpTokenAttributes<Self::BigUint>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id.as_esdt_identifier(),
            token_nonce,
        );

        let attributes = token_info.decode_attributes::<WrappedLpTokenAttributes<Self::BigUint>>();
        match attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn create_and_send(
        &self,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &Self::BigUint,
        locked_token_id: &TokenIdentifier,
        locked_tokens_consumed: &Self::BigUint,
        locked_tokens_nonce: Nonce,
        caller: &Address,
        proxy_params: &ProxyPairParams,
    ) {
        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
        let nonce = self.create_tokens(
            &wrapped_lp_token_id,
            lp_token_id,
            lp_token_amount,
            locked_token_id,
            locked_tokens_consumed,
            locked_tokens_nonce,
            proxy_params,
        );
        self.send()
            .transfer_tokens(&wrapped_lp_token_id, nonce, &lp_token_amount, caller);
    }

    fn create_tokens(
        &self,
        wrapped_lp_token_id: &TokenIdentifier,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &Self::BigUint,
        locked_token_id: &TokenIdentifier,
        locked_tokens_consumed: &Self::BigUint,
        locked_tokens_nonce: Nonce,
        proxy_params: &ProxyPairParams,
    ) -> Nonce {
        let attributes = WrappedLpTokenAttributes::<Self::BigUint> {
            lp_token_id: lp_token_id.clone(),
            lp_token_total_amount: lp_token_amount.clone(),
            locked_assets_token_id: locked_token_id.clone(),
            locked_assets_invested: locked_tokens_consumed.clone(),
            locked_assets_nonce: locked_tokens_nonce,
        };
        self.send()
            .esdt_nft_create::<WrappedLpTokenAttributes<Self::BigUint>>(
                proxy_params.mint_tokens_gas_limit,
                wrapped_lp_token_id.as_esdt_identifier(),
                lp_token_amount,
                &BoxedBytes::empty(),
                &Self::BigUint::zero(),
                &BoxedBytes::empty(),
                &attributes,
                &[BoxedBytes::empty()],
            );
        self.increase_wrapped_lp_token_nonce()
    }

    fn send_temporary_funds_back(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) {
        let amount = self.temporary_funds(caller, token_id, token_nonce).get();
        self.send()
            .transfer_tokens(token_id, token_nonce, &amount, caller);
        self.temporary_funds(caller, token_id, token_nonce).clear();
    }

    fn forward_to_pair(
        &self,
        pair_address: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        amount: &Self::BigUint,
        proxy_params: &ProxyPairParams,
    ) {
        let token_to_send: TokenIdentifier;
        if token_nonce == 0 {
            token_to_send = token_id.clone();
        } else {
            let asset_token_id = self.asset_token_id().get();
            self.send().esdt_local_mint(
                proxy_params.mint_tokens_gas_limit,
                &asset_token_id.as_esdt_identifier(),
                amount,
            );
            token_to_send = asset_token_id;
        };
        self.pair_contract_proxy(pair_address.clone())
            .acceptEsdtPayment(token_to_send, amount.clone())
            .execute_on_dest_context(proxy_params.accept_esdt_payment_gas_limit);
    }

    fn increase_temporary_funds_amount(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        increase_amount: &Self::BigUint,
    ) {
        let old_amount = self.temporary_funds(caller, token_id, token_nonce).get();
        let new_amount = old_amount + increase_amount.clone();
        self.temporary_funds(caller, token_id, token_nonce)
            .set(&new_amount);
    }

    fn increase_wrapped_lp_token_nonce(&self) -> Nonce {
        let new_nonce = self.wrapped_lp_token_nonce().get() + 1;
        self.wrapped_lp_token_nonce().set(&new_nonce);
        new_nonce
    }

    fn decrease_temporary_funds_amount(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        decrease_amount: &Self::BigUint,
    ) {
        let old_amount = self.temporary_funds(caller, token_id, token_nonce).get();
        let new_amount = old_amount - decrease_amount.clone();
        if new_amount > 0 {
            self.temporary_funds(caller, token_id, token_nonce)
                .set(&new_amount);
        } else {
            self.temporary_funds(caller, token_id, token_nonce).clear();
        }
    }

    fn require_is_intermediated_pair(&self, address: &Address) -> SCResult<()> {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
        Ok(())
    }

    fn require_proxy_pair_params_not_empty(&self) -> SCResult<()> {
        require!(!self.proxy_pair_params().is_empty(), "Empty proxy_params");
        Ok(())
    }

    fn require_wrapped_lp_token_id_not_empty(&self) -> SCResult<()> {
        require!(!self.wrapped_lp_token_id().is_empty(), "Empty token id");
        Ok(())
    }

    #[view(getTemporaryFunds)]
    #[storage_mapper("funds")]
    fn temporary_funds(
        &self,
        caller: &Address,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
    ) -> SingleValueMapper<Self::Storage, Self::BigUint>;

    #[view(getIntermediatedPairs)]
    #[storage_mapper("intermediated_pairs")]
    fn intermediated_pairs(&self) -> SetMapper<Self::Storage, Address>;

    #[view(getWrappedLpTokenId)]
    #[storage_mapper("wrapped_lp_token_id")]
    fn wrapped_lp_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("wrapped_tp_token_nonce")]
    fn wrapped_lp_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;

    #[storage_mapper("proxy_pair_params")]
    fn proxy_pair_params(&self) -> SingleValueMapper<Self::Storage, ProxyPairParams>;
}
