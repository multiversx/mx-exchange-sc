elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::locked_token::{LockedTokenAttributes, PreviousStatusFlag, UnlockedPaymentWrapper};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct LpProxyTokenAttributes<M: ManagedTypeApi> {
    pub lp_address: ManagedAddress<M>,
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

#[elrond_wasm::module]
pub trait ProxyLpModule:
    crate::locked_token::LockedTokenModule
    + crate::lp_interactions::LpInteractionsModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
        let payment_amount = self.call_value().egld_value();

        self.lp_proxy_token().issue(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[only_owner]
    #[endpoint(setLocalRolesLpProxyToken)]
    fn set_local_roles_lp_proxy_token(&self) {
        self.lp_proxy_token().set_local_roles(
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
            ],
            None,
        );
    }

    #[only_owner]
    #[endpoint(addLpToWhitelist)]
    fn add_lp_to_whitelist(&self, lp_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&lp_address),
            "Invalid LP address"
        );

        self.lp_address_whitelist().add(&lp_address);
    }

    #[only_owner]
    #[endpoint(removeLpFromWhitelist)]
    fn remove_lp_from_whitelist(&self, lp_address: ManagedAddress) {
        self.lp_address_whitelist().remove(&lp_address);
    }

    #[payable("*")]
    #[endpoint(addLiquidityLockedToken)]
    fn add_liquidity_locked_token(
        &self,
        lp_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityThroughProxyResultType<Self::Api> {
        self.lp_address_whitelist().require_whitelisted(&lp_address);

        let payments = self.call_value().all_esdt_transfers();
        let (first_payment_unlocked_wrapper, second_payment_unlocked_wrapper) =
            self.unlock_lp_payments(payments);

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
            "Only locked tokens with fungible original tokens can be used as liquidity"
        );
        require!(
            ref_first_payment_unlocked.token_identifier
                != ref_second_payment_unlocked.token_identifier,
            "Must use two different original tokens for add liquidity"
        );

        let add_liq_result = self.call_pair_add_liquidity(
            lp_address.clone(),
            ref_first_payment_unlocked.clone(),
            ref_second_payment_unlocked.clone(),
            first_token_amount_min,
            second_token_amount_min,
        );

        let caller = self.blockchain().get_caller();
        let first_token_refund_payment = self.lock_if_needed_and_send(
            &caller,
            add_liq_result.first_token_refund,
            first_payment_unlocked_wrapper.status_before,
        );
        let second_token_refund_payment = self.lock_if_needed_and_send(
            &caller,
            add_liq_result.second_token_refund,
            second_payment_unlocked_wrapper.status_before,
        );

        let proxy_token_attributes = self.create_lp_proxy_token_attributes(
            lp_address,
            add_liq_result.lp_tokens.token_identifier,
            first_payment_unlocked_wrapper,
            second_payment_unlocked_wrapper,
        );
        let lp_proxy_payment = self.lp_proxy_token().nft_create_and_send(
            &caller,
            add_liq_result.lp_tokens.amount,
            &proxy_token_attributes,
        );

        (
            first_token_refund_payment,
            second_token_refund_payment,
            lp_proxy_payment,
        )
            .into()
    }

    #[payable("*")]
    #[endpoint(removeLiquidityLockedToken)]
    fn remove_liquidity_locked_token(
        &self,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> RemoveLiquidityThroughProxyResultType<Self::Api> {
        let payment: EsdtTokenPayment<Self::Api> = self.call_value().payment();
        let lp_proxy_token_mapper = self.lp_proxy_token();
        lp_proxy_token_mapper.require_same_token(&payment.token_identifier);

        let lp_proxy_token_attributes: LpProxyTokenAttributes<Self::Api> =
            lp_proxy_token_mapper.get_token_attributes(payment.token_nonce);
        lp_proxy_token_mapper.nft_burn(payment.token_nonce, &payment.amount);

        let lp_token_amount = payment.amount;
        let remove_liq_result = self.call_pair_remove_liquidity(
            lp_proxy_token_attributes.lp_address,
            lp_proxy_token_attributes.lp_token_id,
            lp_token_amount,
            first_token_amount_min,
            second_token_amount_min,
            lp_proxy_token_attributes.first_token_id.clone(),
        );

        let caller = self.blockchain().get_caller();
        let first_token_result_payment = self.lock_if_needed_and_send(
            &caller,
            remove_liq_result.first_token_payment_out,
            PreviousStatusFlag::new(lp_proxy_token_attributes.first_token_locked_nonce),
        );
        let second_token_result_payment = self.lock_if_needed_and_send(
            &caller,
            remove_liq_result.second_token_payment_out,
            PreviousStatusFlag::new(lp_proxy_token_attributes.second_token_locked_nonce),
        );

        (first_token_result_payment, second_token_result_payment).into()
    }

    fn unlock_lp_payments(
        &self,
        payments: ManagedVec<Self::Api, EsdtTokenPayment<Self::Api>>,
    ) -> (
        UnlockedPaymentWrapper<Self::Api>,
        UnlockedPaymentWrapper<Self::Api>,
    ) {
        require!(
            payments.len() == 2,
            "Invalid number of payments for add liquidity"
        );

        let first_payment = payments.get(0);
        let second_payment = payments.get(1);

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
                attributes.original_token_id,
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
        lp_address: ManagedAddress,
        lp_token_id: TokenIdentifier,
        first_payment_unlocked_wrapper: UnlockedPaymentWrapper<Self::Api>,
        second_payment_unlocked_wrapper: UnlockedPaymentWrapper<Self::Api>,
    ) -> LpProxyTokenAttributes<Self::Api> {
        let first_token_locked_nonce = first_payment_unlocked_wrapper.get_locked_token_nonce();
        let first_token_id = first_payment_unlocked_wrapper.payment.token_identifier;
        let second_token_locked_nonce = second_payment_unlocked_wrapper.get_locked_token_nonce();
        let second_token_id = second_payment_unlocked_wrapper.payment.token_identifier;

        LpProxyTokenAttributes {
            lp_address,
            lp_token_id,
            first_token_id,
            first_token_locked_nonce,
            second_token_id,
            second_token_locked_nonce,
        }
    }

    fn lock_if_needed_and_send(
        &self,
        to: &ManagedAddress,
        payment: EsdtTokenPayment<Self::Api>,
        prev_status: PreviousStatusFlag,
    ) -> EsdtTokenPayment<Self::Api> {
        if payment.amount == 0 {
            return payment;
        }

        match prev_status {
            PreviousStatusFlag::NotLocked => {
                self.send().direct(
                    to,
                    &payment.token_identifier,
                    payment.token_nonce,
                    &payment.amount,
                    &[],
                );

                payment
            }
            PreviousStatusFlag::Locked { locked_token_nonce } => self
                .locked_token()
                .nft_add_quantity_and_send(to, locked_token_nonce, payment.amount),
        }
    }

    #[storage_mapper("lpAddressWhitelist")]
    fn lp_address_whitelist(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;

    #[view(getLpProxyTokenId)]
    #[storage_mapper("lpProxyTokenId")]
    fn lp_proxy_token(&self) -> NonFungibleTokenMapper<Self::Api>;
}
