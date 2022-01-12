#![allow(clippy::too_many_arguments)]
#![allow(clippy::comparison_chain)]
#![allow(clippy::vec_init_then_push)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use proxy_common::ACCEPT_PAY_FUNC_NAME;

use common_structs::{Nonce, WrappedLpTokenAttributes};
use pair::config::ProxyTrait as _;

use super::events;
use super::proxy_common;
use super::wrapped_lp_token_merge;

type AddLiquidityResultType<BigUint> =
    MultiResult3<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

type RemoveLiquidityResultType<BigUint> =
    MultiResult2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

#[derive(Clone, ManagedVecItem)]
pub struct WrappedLpToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: WrappedLpTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait ProxyPairModule:
    proxy_common::ProxyCommonModule
    + token_supply::TokenSupplyModule
    + wrapped_lp_token_merge::WrappedLpTokenMerge
    + token_merge::TokenMergeModule
    + token_send::TokenSendModule
    + events::EventsModule
{
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[only_owner]
    #[endpoint(addPairToIntermediate)]
    fn add_pair_to_intermediate(&self, pair_address: ManagedAddress) {
        self.intermediated_pairs().insert(pair_address);
    }

    #[only_owner]
    #[endpoint(removeIntermediatedPair)]
    fn remove_intermediated_pair(&self, pair_address: ManagedAddress) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.intermediated_pairs().remove(&pair_address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(addLiquidityProxy)]
    fn add_liquidity_proxy(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.require_wrapped_lp_token_id_not_empty()?;

        let first_payment = match payments.get(0) {
            Some(p) => p,
            None => return sc_error!("bad payment len"),
        };
        let second_payment = match payments.get(1) {
            Some(p) => p,
            None => return sc_error!("bad payment len"),
        };

        let first_token_id = first_payment.token_identifier.clone();
        let first_token_nonce = first_payment.token_nonce;
        let first_token_amount_desired = first_payment.amount.clone();
        require!(first_token_nonce == 0, "bad first token nonce");
        require!(
            first_token_amount_desired > 0u32,
            "first payment amount zero"
        );
        require!(
            first_token_amount_desired > first_token_amount_min,
            "bad first token min"
        );

        let second_token_id = second_payment.token_identifier.clone();
        let second_token_nonce = second_payment.token_nonce;
        let second_token_amount_desired = second_payment.amount.clone();
        require!(second_token_nonce != 0, "bad second token nonce");
        require!(
            second_token_amount_desired > 0u32,
            "second payment amount zero"
        );
        require!(
            second_token_amount_desired > second_token_amount_min,
            "bad second token min"
        );

        let asset_token_id = self.asset_token_id().get();
        self.mint_tokens(&asset_token_id, &second_token_amount_desired);

        let result = self.actual_add_liquidity(
            &pair_address,
            &first_token_id,
            &first_token_amount_desired,
            &first_token_amount_min,
            &asset_token_id,
            &second_token_amount_desired,
            &second_token_amount_min,
        );

        let result_tuple = result.0;
        let lp_received = result_tuple.0;
        let first_token_used = result_tuple.1;
        let second_token_used = result_tuple.2;
        require!(
            lp_received.amount > 0u32,
            "LP token amount should be greater than 0"
        );
        require!(
            first_token_used.amount <= first_token_amount_desired,
            "Used more first tokens than provided"
        );
        require!(
            second_token_used.amount <= second_token_amount_desired,
            "Used more second tokens than provided"
        );

        let caller = self.blockchain().get_caller();
        let additional_payments = match payments.slice(2, payments.len()) {
            Some(p) => p,
            None => ManagedVec::new(),
        };
        let (new_wrapped_lp_token, created_with_merge) = self.create_by_merging_and_send(
            &lp_received.token_identifier,
            &lp_received.amount,
            &second_token_used.amount,
            second_token_nonce,
            &caller,
            &additional_payments,
        )?;

        let mut surplus_payments = ManagedVec::new();
        surplus_payments.push(EsdtTokenPayment::new(
            first_token_id.clone(),
            0,
            &first_token_amount_desired - &first_token_used.amount,
        ));
        surplus_payments.push(EsdtTokenPayment::new(
            second_token_id.clone(),
            second_token_nonce,
            &second_token_amount_desired - &second_token_used.amount,
        ));
        self.send_multiple_tokens(&caller, &surplus_payments, OptionalArg::None)?;

        if second_token_amount_desired > second_token_used.amount {
            let unused_minted_assets = &second_token_amount_desired - &second_token_used.amount;
            self.burn_tokens(&asset_token_id, &unused_minted_assets);
        }

        self.emit_add_liquidity_proxy_event(
            &caller,
            &pair_address,
            &first_token_id,
            first_token_nonce,
            &first_token_used.amount,
            &second_token_id,
            first_token_nonce,
            &second_token_used.amount,
            &new_wrapped_lp_token.token_amount.token_identifier,
            new_wrapped_lp_token.token_amount.token_nonce,
            &new_wrapped_lp_token.token_amount.amount,
            &new_wrapped_lp_token.attributes,
            created_with_merge,
        );
        Ok(())
    }

    #[payable("*")]
    #[endpoint(removeLiquidityProxy)]
    fn remove_liquidity_proxy(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: BigUint,
        #[payment_nonce] token_nonce: Nonce,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> SCResult<()> {
        self.require_is_intermediated_pair(&pair_address)?;
        self.require_wrapped_lp_token_id_not_empty()?;
        require!(token_nonce != 0, "Can only be called with an SFT");
        require!(amount != 0u32, "Payment amount cannot be zero");

        let wrapped_lp_token_id = self.wrapped_lp_token_id().get();
        require!(token_id == wrapped_lp_token_id, "Wrong input token");

        let caller = self.blockchain().get_caller();
        let lp_token_id = self.ask_for_lp_token_id(&pair_address);
        let attributes = self.get_wrapped_lp_token_attributes(&token_id, token_nonce)?;
        require!(lp_token_id == attributes.lp_token_id, "Bad input address");

        let locked_asset_token_id = self.locked_asset_token_id().get();
        let asset_token_id = self.asset_token_id().get();

        let tokens_for_position = self
            .actual_remove_liquidity(
                &pair_address,
                &lp_token_id,
                &amount,
                &first_token_amount_min,
                &second_token_amount_min,
            )
            .into_tuple();

        let fungible_token_id: TokenIdentifier;
        let fungible_token_amount: BigUint;
        let assets_received: BigUint;
        let locked_assets_invested = self.rule_of_three_non_zero_result(
            &amount,
            &attributes.lp_token_total_amount,
            &attributes.locked_assets_invested,
        )?;

        if tokens_for_position.1.token_identifier == asset_token_id {
            assets_received = tokens_for_position.1.amount.clone();
            fungible_token_id = tokens_for_position.0.token_identifier.clone();
            fungible_token_amount = tokens_for_position.0.amount.clone();
        } else {
            return sc_error!("Bad tokens received from pair SC");
        }

        //Send back the tokens removed from pair sc.
        self.send()
            .direct(&caller, &fungible_token_id, 0, &fungible_token_amount, &[]);
        let locked_assets_to_send =
            core::cmp::min(assets_received.clone(), locked_assets_invested.clone());
        self.transfer_execute_custom(
            &caller,
            &locked_asset_token_id,
            attributes.locked_assets_nonce,
            &locked_assets_to_send,
            OptionalArg::None,
        )?;

        //Do cleanup
        if assets_received > locked_assets_invested {
            let difference = assets_received - locked_assets_invested;
            self.send()
                .direct(&caller, &asset_token_id, 0, &difference, &[]);
        } else if assets_received < locked_assets_invested {
            let difference = locked_assets_invested - assets_received;
            self.nft_burn_tokens(
                &locked_asset_token_id,
                attributes.locked_assets_nonce,
                &difference,
            );
        }

        self.burn_tokens(&asset_token_id, &locked_assets_to_send);
        self.nft_burn_tokens(&wrapped_lp_token_id, token_nonce, &amount);

        self.emit_remove_liquidity_proxy_event(
            &caller,
            &pair_address,
            &token_id,
            token_nonce,
            &amount,
            &attributes,
            &tokens_for_position.0.token_identifier,
            0,
            &tokens_for_position.0.amount,
            &tokens_for_position.1.token_identifier,
            0,
            &tokens_for_position.1.amount,
        );
        Ok(())
    }

    fn actual_add_liquidity(
        &self,
        pair_address: &ManagedAddress,
        first_token_id: &TokenIdentifier,
        first_token_amount_desired: &BigUint,
        first_token_amount_min: &BigUint,
        second_token_id: &TokenIdentifier,
        second_token_amount_desired: &BigUint,
        second_token_amount_min: &BigUint,
    ) -> AddLiquidityResultType<Self::Api> {
        let mut all_token_payments = ManagedVec::new();

        let first_payment = EsdtTokenPayment::new(
            first_token_id.clone(),
            0,
            first_token_amount_desired.clone(),
        );
        all_token_payments.push(first_payment);

        let second_payment = EsdtTokenPayment::new(
            second_token_id.clone(),
            0,
            second_token_amount_desired.clone(),
        );
        all_token_payments.push(second_payment);

        self.pair_contract_proxy(pair_address.clone())
            .add_liquidity(
                all_token_payments,
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
                OptionalArg::Some(ManagedBuffer::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
    }

    fn actual_remove_liquidity(
        &self,
        pair_address: &ManagedAddress,
        lp_token_id: &TokenIdentifier,
        liquidity: &BigUint,
        first_token_amount_min: &BigUint,
        second_token_amount_min: &BigUint,
    ) -> RemoveLiquidityResultType<Self::Api> {
        self.pair_contract_proxy(pair_address.clone())
            .remove_liquidity(
                lp_token_id.clone(),
                liquidity.clone(),
                first_token_amount_min.clone(),
                second_token_amount_min.clone(),
                OptionalArg::Some(ManagedBuffer::from(ACCEPT_PAY_FUNC_NAME)),
            )
            .execute_on_dest_context()
    }

    fn ask_for_lp_token_id(&self, pair_address: &ManagedAddress) -> TokenIdentifier {
        self.pair_contract_proxy(pair_address.clone())
            .get_lp_token_identifier()
            .execute_on_dest_context()
    }

    fn create_by_merging_and_send(
        &self,
        lp_token_id: &TokenIdentifier,
        lp_token_amount: &BigUint,
        locked_tokens_consumed: &BigUint,
        locked_tokens_nonce: Nonce,
        caller: &ManagedAddress,
        additional_payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<(WrappedLpToken<Self::Api>, bool)> {
        self.merge_wrapped_lp_tokens_and_send(
            caller,
            additional_payments,
            Option::Some(WrappedLpToken {
                token_amount: self.create_payment(
                    &self.wrapped_lp_token_id().get(),
                    0,
                    lp_token_amount,
                ),
                attributes: WrappedLpTokenAttributes {
                    lp_token_id: lp_token_id.clone(),
                    lp_token_total_amount: lp_token_amount.clone(),
                    locked_assets_invested: locked_tokens_consumed.clone(),
                    locked_assets_nonce: locked_tokens_nonce,
                },
            }),
            OptionalArg::None,
        )
    }

    fn require_is_intermediated_pair(&self, address: &ManagedAddress) -> SCResult<()> {
        require!(
            self.intermediated_pairs().contains(address),
            "Not an intermediated pair"
        );
        Ok(())
    }

    fn require_wrapped_lp_token_id_not_empty(&self) -> SCResult<()> {
        require!(!self.wrapped_lp_token_id().is_empty(), "Empty token id");
        Ok(())
    }
}
