elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::locked_token::{LockedTokenAttributes, PreviousStatusFlag, UnlockedPaymentWrapper};

pub type AddLiquidityResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

pub struct AddLiquidityResultWrapper<M: ManagedTypeApi> {
    pub lp_tokens: EsdtTokenPayment<M>,
    pub first_token_refund: EsdtTokenPayment<M>,
    pub second_token_refund: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct LpProxyTokenAttributes<M: ManagedTypeApi> {
    pub lp_address: ManagedAddress<M>,
    pub lp_token_id: TokenIdentifier<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub first_token_locked_nonce: u64,
    pub second_token_id: TokenIdentifier<M>,
    pub second_token_locked_nonce: u64,
}

// Must manually declare, as Pair SC already depends on simple-lock
// This avoids circular dependency
pub mod lp_proxy {
    elrond_wasm::imports!();
    use super::{AddLiquidityResultType, RemoveLiquidityResultType};

    #[elrond_wasm::proxy]
    pub trait LpProxy {
        #[payable("*")]
        #[endpoint(addLiquidity)]
        fn add_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> AddLiquidityResultType<Self::Api>;

        #[payable("*")]
        #[endpoint(removeLiquidity)]
        fn remove_liquidity(
            &self,
            first_token_amount_min: BigUint,
            second_token_amount_min: BigUint,
        ) -> RemoveLiquidityResultType<Self::Api>;
    }
}

#[elrond_wasm::module]
pub trait ProxyLpModule:
    crate::locked_token::LockedTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueProxyToken)]
    fn issue_proxy_token(
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
    #[endpoint(setLocalRolesProxyToken)]
    fn set_local_roles_proxy_token(&self) {
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
    #[endpoint]
    fn add_liquidity_locked_token(
        &self,
        lp_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> EsdtTokenPayment<Self::Api> {
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
        self.lock_if_needed_and_send(
            &caller,
            add_liq_result.first_token_refund,
            first_payment_unlocked_wrapper.status_before,
        );
        self.lock_if_needed_and_send(
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
        self.lp_proxy_token().nft_create_and_send(
            &caller,
            add_liq_result.lp_tokens.amount,
            &proxy_token_attributes,
        )
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

    fn call_pair_add_liquidity(
        &self,
        lp_address: ManagedAddress,
        first_payment: EsdtTokenPayment<Self::Api>,
        second_payment: EsdtTokenPayment<Self::Api>,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityResultWrapper<Self::Api> {
        let second_token_id_copy = second_payment.token_identifier.clone();

        let mut lp_payments_in = ManagedVec::new();
        lp_payments_in.push(first_payment);
        lp_payments_in.push(second_payment);

        let lp_payments_out: AddLiquidityResultType<Self::Api> = self
            .lp_proxy(lp_address.clone())
            .add_liquidity(first_token_amount_min, second_token_amount_min)
            .with_multi_token_transfer(lp_payments_in)
            .execute_on_dest_context_custom_range(|_, after| (after - 3, after));
        let (lp_tokens, mut first_token_refund, mut second_token_refund) =
            lp_payments_out.into_tuple();

        if first_token_refund.token_identifier == second_token_id_copy {
            core::mem::swap(&mut first_token_refund, &mut second_token_refund);
        }

        AddLiquidityResultWrapper {
            lp_tokens,
            first_token_refund,
            second_token_refund,
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
    ) {
        if payment.amount == 0 {
            return;
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
            }
            PreviousStatusFlag::Locked { locked_token_nonce } => {
                self.locked_token().nft_add_quantity_and_send(
                    to,
                    locked_token_nonce,
                    payment.amount,
                );
            }
        }
    }

    #[proxy]
    fn lp_proxy(&self, sc_address: ManagedAddress) -> lp_proxy::Proxy<Self::Api>;

    #[storage_mapper("lpAddressWhitelist")]
    fn lp_address_whitelist(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;

    #[view(getLpProxyTokenId)]
    #[storage_mapper("lpProxyTokenId")]
    fn lp_proxy_token(&self) -> NonFungibleTokenMapper<Self::Api>;
}
