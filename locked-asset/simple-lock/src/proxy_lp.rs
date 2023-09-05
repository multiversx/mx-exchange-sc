multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::error_messages::*;
use crate::locked_token::{LockedTokenAttributes, PreviousStatusFlag, UnlockedPaymentWrapper};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct LpProxyTokenAttributes<M: ManagedTypeApi> {
    pub lp_token_id: TokenIdentifier<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub first_token_locked_nonce: u64,
    pub second_token_id: TokenIdentifier<M>,
    pub second_token_locked_nonce: u64,
}

pub type AddLiquidityThroughProxyResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type RemoveLiquidityThroughProxyResultType<M> =
    MultiValue2<EsdtTokenPayment<M>, EsdtTokenPayment<M>>;

#[multiversx_sc::module]
pub trait ProxyLpModule:
    crate::locked_token::LockedTokenModule
    + crate::lp_interactions::LpInteractionsModule
    + crate::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueLpProxyToken)]
    fn issue_lp_proxy_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value().clone_value();

        self.lp_proxy_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    /// Add a liquidity pool to the whitelist.
    /// If the token pair does not have an associated pool, users may not add liquidity.
    ///
    /// `first_token_id` and `second_token_id` MUST match the LP's order,
    /// otherwise all attempts at adding liquidity will fail
    ///
    /// May not add pools for both pairs, i.e. (first, second) and (second, first)
    #[only_owner]
    #[endpoint(addLpToWhitelist)]
    fn add_lp_to_whitelist(
        &self,
        lp_address: ManagedAddress,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) {
        require!(
            self.blockchain().is_smart_contract(&lp_address),
            INVALID_SC_ADDRESS_ERR_MSG
        );
        require!(
            first_token_id != second_token_id,
            MUST_USE_DIFFERENT_TOKENS_ERR_MSG
        );
        require!(
            first_token_id.is_valid_esdt_identifier() && second_token_id.is_valid_esdt_identifier(),
            "Only ESDT tokens accepted"
        );
        require!(
            self.lp_address_for_token_pair(&second_token_id, &first_token_id)
                .is_empty(),
            "Address already set for the reverse token pair"
        );

        self.lp_address_for_token_pair(&first_token_id, &second_token_id)
            .set(&lp_address);

        let is_new_lp = self.known_liquidity_pools().insert(lp_address);
        require!(is_new_lp, "Liquidty Pool address already known");
    }

    /// Removes a liquidity pool from the whitelist, for the selected token pair.
    #[only_owner]
    #[endpoint(removeLpFromWhitelist)]
    fn remove_lp_from_whitelist(
        &self,
        lp_address: ManagedAddress,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
    ) {
        let was_removed = self.known_liquidity_pools().swap_remove(&lp_address);
        require!(was_removed, "Liquidty Pool address not known");

        let correct_order_mapper =
            self.lp_address_for_token_pair(&first_token_id, &second_token_id);
        let reverse_order_mapper =
            self.lp_address_for_token_pair(&second_token_id, &first_token_id);

        if !correct_order_mapper.is_empty() {
            let stored_lp_addr = correct_order_mapper.take();
            require!(stored_lp_addr == lp_address, LP_REMOVAL_WRONG_PAIR);
        } else if !reverse_order_mapper.is_empty() {
            let stored_lp_addr = reverse_order_mapper.take();
            require!(stored_lp_addr == lp_address, LP_REMOVAL_WRONG_PAIR);
        } else {
            sc_panic!(LP_REMOVAL_WRONG_PAIR);
        }
    }

    /// Add liquidity through a LOCKED token.
    /// Will fail if a liquidity pool is not configured for the token pair.
    ///
    /// Expected payments: Any one of the following pairs:
    /// - (LOCKED token, LOCKED token)
    /// - (LOCKED token, any token)
    /// - (any token, LOCKED token)
    ///
    /// Arguments: first_token_amount_min, second_token_amount_min - Arguments forwarded to the LP pool.
    /// May not be zero.
    ///
    /// Output payments:
    /// - refunded tokens from the first payment
    /// - refunded tokens from the second payment
    /// - LP_PROXY tokens, which can later be used to further interact with the LP pool through this SC
    #[payable("*")]
    #[endpoint(addLiquidityLockedToken)]
    fn add_liquidity_locked_token(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityThroughProxyResultType<Self::Api> {
        let [first_payment, second_payment] = self.call_value().multi_esdt();
        let (mut first_payment_unlocked_wrapper, mut second_payment_unlocked_wrapper) =
            self.unlock_lp_payments(first_payment, second_payment);
        let lp_address = self.try_get_lp_address_and_fix_token_order(
            &mut first_payment_unlocked_wrapper,
            &mut second_payment_unlocked_wrapper,
        );

        require!(
            first_payment_unlocked_wrapper.status_before.was_locked()
                || second_payment_unlocked_wrapper.status_before.was_locked(),
            "At least one of the payments must be a locked token"
        );

        let ref_first_payment_unlocked = &first_payment_unlocked_wrapper.payment;
        let ref_second_payment_unlocked = &second_payment_unlocked_wrapper.payment;

        require!(
            ref_first_payment_unlocked.token_nonce == 0
                && ref_second_payment_unlocked.token_nonce == 0,
            ONLY_FUNGIBLE_TOKENS_ALLOWED_ERR_MSG
        );
        require!(
            ref_first_payment_unlocked.token_identifier
                != ref_second_payment_unlocked.token_identifier,
            MUST_USE_DIFFERENT_TOKENS_ERR_MSG
        );

        let add_liq_result = self.call_pair_add_liquidity(
            lp_address,
            ref_first_payment_unlocked,
            ref_second_payment_unlocked,
            first_token_amount_min,
            second_token_amount_min,
        );

        let caller = self.blockchain().get_caller();
        let first_token_refund_payment = self.send_tokens_optimal_status(
            &caller,
            add_liq_result.first_token_refund,
            first_payment_unlocked_wrapper.status_before,
        );
        let second_token_refund_payment = self.send_tokens_optimal_status(
            &caller,
            add_liq_result.second_token_refund,
            second_payment_unlocked_wrapper.status_before,
        );

        let lp_token_name = add_liq_result
            .lp_tokens
            .token_identifier
            .as_managed_buffer()
            .clone();
        let proxy_token_attributes = self.create_lp_proxy_token_attributes(
            add_liq_result.lp_tokens.token_identifier,
            first_payment_unlocked_wrapper,
            second_payment_unlocked_wrapper,
        );

        let lp_proxy_token_mapper = self.lp_proxy_token();
        let lp_proxy_nonce = self.get_or_create_nonce_for_attributes(
            &lp_proxy_token_mapper,
            &lp_token_name,
            &proxy_token_attributes,
        );

        let lp_proxy_payment = lp_proxy_token_mapper.nft_add_quantity_and_send(
            &caller,
            lp_proxy_nonce,
            add_liq_result.lp_tokens.amount,
        );

        (
            first_token_refund_payment,
            second_token_refund_payment,
            lp_proxy_payment,
        )
            .into()
    }

    /// Remove liquidity previously added through `addLiquidityLockedToken`.
    /// If the unlock_epoch has not passed for the original LOCKED tokens,
    /// the caller will receive locked tokens. Otherwise, they will receive the unlocked version.
    ///
    /// Expected payments: LP_PROXY tokens
    ///
    /// Arguments: first_token_amount_min, second_token_amount_min - Arguments forwarded to the LP pool.
    /// May not be zero.
    ///
    /// Output payments:
    /// first_token original liquidity + rewards
    /// second_token original liquidity + rewards
    #[payable("*")]
    #[endpoint(removeLiquidityLockedToken)]
    fn remove_liquidity_locked_token(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> RemoveLiquidityThroughProxyResultType<Self::Api> {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().single_esdt();
        let lp_proxy_token_mapper = self.lp_proxy_token();
        lp_proxy_token_mapper.require_same_token(&payment.token_identifier);

        let lp_proxy_token_attributes: LpProxyTokenAttributes<Self::Api> =
            lp_proxy_token_mapper.get_token_attributes(payment.token_nonce);
        lp_proxy_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let lp_address = self
            .lp_address_for_token_pair(
                &lp_proxy_token_attributes.first_token_id,
                &lp_proxy_token_attributes.second_token_id,
            )
            .get();
        let lp_token_amount = payment.amount;
        let remove_liq_result = self.call_pair_remove_liquidity(
            lp_address,
            lp_proxy_token_attributes.lp_token_id,
            lp_token_amount,
            first_token_amount_min,
            second_token_amount_min,
            &lp_proxy_token_attributes.first_token_id,
            &lp_proxy_token_attributes.second_token_id,
        );

        let caller = self.blockchain().get_caller();
        let first_token_result_payment = self.send_tokens_optimal_status(
            &caller,
            remove_liq_result.first_token_payment_out,
            PreviousStatusFlag::new(lp_proxy_token_attributes.first_token_locked_nonce),
        );
        let second_token_result_payment = self.send_tokens_optimal_status(
            &caller,
            remove_liq_result.second_token_payment_out,
            PreviousStatusFlag::new(lp_proxy_token_attributes.second_token_locked_nonce),
        );

        (first_token_result_payment, second_token_result_payment).into()
    }

    fn unlock_lp_payments(
        &self,
        first_payment: EsdtTokenPayment<Self::Api>,
        second_payment: EsdtTokenPayment<Self::Api>,
    ) -> (
        UnlockedPaymentWrapper<Self::Api>,
        UnlockedPaymentWrapper<Self::Api>,
    ) {
        let first_payment_unlocked = self.unlock_single_payment(first_payment);
        let second_payment_unlocked = self.unlock_single_payment(second_payment);

        (first_payment_unlocked, second_payment_unlocked)
    }

    fn unlock_single_payment(
        &self,
        payment: EsdtTokenPayment<Self::Api>,
    ) -> UnlockedPaymentWrapper<Self::Api> {
        let locked_token_mapper = self.locked_token();
        let locked_token_id = locked_token_mapper.get_token_id();

        if payment.token_identifier == locked_token_id {
            let attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(payment.token_nonce);

            locked_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

            let unlocked_payment = EsdtTokenPayment::new(
                attributes.original_token_id.unwrap_esdt(),
                attributes.original_token_nonce,
                payment.amount,
            );

            UnlockedPaymentWrapper {
                payment: unlocked_payment,
                status_before: PreviousStatusFlag::Locked {
                    locked_token_nonce: payment.token_nonce,
                },
            }
        } else {
            UnlockedPaymentWrapper {
                payment,
                status_before: PreviousStatusFlag::NotLocked,
            }
        }
    }

    fn create_lp_proxy_token_attributes(
        &self,
        lp_token_id: TokenIdentifier,
        first_payment_unlocked_wrapper: UnlockedPaymentWrapper<Self::Api>,
        second_payment_unlocked_wrapper: UnlockedPaymentWrapper<Self::Api>,
    ) -> LpProxyTokenAttributes<Self::Api> {
        let first_token_locked_nonce = first_payment_unlocked_wrapper.get_locked_token_nonce();
        let first_token_id = first_payment_unlocked_wrapper.payment.token_identifier;
        let second_token_locked_nonce = second_payment_unlocked_wrapper.get_locked_token_nonce();
        let second_token_id = second_payment_unlocked_wrapper.payment.token_identifier;

        LpProxyTokenAttributes {
            lp_token_id,
            first_token_id,
            first_token_locked_nonce,
            second_token_id,
            second_token_locked_nonce,
        }
    }

    fn try_get_lp_address_and_fix_token_order(
        &self,
        first_unlocked_payment: &mut UnlockedPaymentWrapper<Self::Api>,
        second_unlocked_payment: &mut UnlockedPaymentWrapper<Self::Api>,
    ) -> ManagedAddress {
        let correct_order_mapper = self.lp_address_for_token_pair(
            &first_unlocked_payment.payment.token_identifier,
            &second_unlocked_payment.payment.token_identifier,
        );
        if !correct_order_mapper.is_empty() {
            return correct_order_mapper.get();
        }

        let reverse_order_mapper = self.lp_address_for_token_pair(
            &second_unlocked_payment.payment.token_identifier,
            &first_unlocked_payment.payment.token_identifier,
        );
        require!(
            !reverse_order_mapper.is_empty(),
            "No LP address for token pair"
        );

        core::mem::swap(first_unlocked_payment, second_unlocked_payment);
        reverse_order_mapper.get()
    }

    #[view(getKnownLiquidityPools)]
    #[storage_mapper("knownLiquidityPools")]
    fn known_liquidity_pools(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("lpAddressForTokenPair")]
    fn lp_address_for_token_pair(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
    ) -> SingleValueMapper<ManagedAddress>;

    #[view(getLpProxyTokenId)]
    #[storage_mapper("lpProxyTokenId")]
    fn lp_proxy_token(&self) -> NonFungibleTokenMapper;
}
