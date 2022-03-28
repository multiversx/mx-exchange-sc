elrond_wasm::imports!();

use crate::{
    locked_token::{LockedTokenAttributes, PreviousStatusFlag, UnlockedPaymentWrapper},
    proxy_token::ProxyTokenAttributes,
};

pub type AddLiquidityResultType<M> =
    MultiValue3<EsdtTokenPayment<M>, EsdtTokenPayment<M>, EsdtTokenPayment<M>>;
pub type RemoveLiquidityResultType<BigUint> =
    MultiValue2<EsdtTokenPayment<BigUint>, EsdtTokenPayment<BigUint>>;

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
    + crate::proxy_token::ProxyTokenModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
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
            self.unlock_payments(payments);

        require!(
            first_payment_unlocked_wrapper.status_before.was_locked()
                || second_payment_unlocked_wrapper.status_before.was_locked(),
            "At least one of the payments must be a locked token"
        );

        let first_payment_unlocked = &first_payment_unlocked_wrapper.payment;
        let second_payment_unlocked = &second_payment_unlocked_wrapper.payment;

        require!(
            first_payment_unlocked.token_nonce == 0 && second_payment_unlocked.token_nonce == 0,
            "Only locked tokens with fungible original tokens can be used as liquidity"
        );
        require!(
            first_payment_unlocked.token_identifier != second_payment_unlocked.token_identifier,
            "Must use two different original tokens for add liquidity"
        );

        let mut lp_payments_in = ManagedVec::new();
        lp_payments_in.push(first_payment_unlocked.clone());
        lp_payments_in.push(second_payment_unlocked.clone());

        let lp_payments_out: AddLiquidityResultType<Self::Api> = self
            .lp_proxy(lp_address.clone())
            .add_liquidity(first_token_amount_min, second_token_amount_min)
            .with_multi_token_transfer(lp_payments_in)
            .execute_on_dest_context_custom_range(|_, after| (after - 3, after));
        let (lp_tokens, mut first_token_refund, mut second_token_refund) =
            lp_payments_out.into_tuple();

        if first_token_refund.token_identifier == second_payment_unlocked.token_identifier {
            core::mem::swap(&mut first_token_refund, &mut second_token_refund);
        }

        let first_payment_unlock_epoch = first_payment_unlocked_wrapper
            .status_before
            .get_unlock_epoch();
        let second_payment_unlock_epoch = second_payment_unlocked_wrapper
            .status_before
            .get_unlock_epoch();

        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        self.lock_and_refund(
            &caller,
            first_token_refund,
            current_epoch,
            first_payment_unlock_epoch,
        );
        self.lock_and_refund(
            &caller,
            second_token_refund,
            current_epoch,
            second_payment_unlock_epoch,
        );

        let max_unlock_epoch =
            core::cmp::max(first_payment_unlock_epoch, second_payment_unlock_epoch);
        let proxy_token_attributes = ProxyTokenAttributes::LpProxyToken {
            lp_address,
            lp_token_id: lp_tokens.token_identifier,
            unlock_epoch: max_unlock_epoch,
        };
        self.proxy_token()
            .nft_create_and_send(&caller, lp_tokens.amount, &proxy_token_attributes)
    }

    fn unlock_payments(
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
                    unlock_epoch: attributes.unlock_epoch,
                },
            }
        } else {
            UnlockedPaymentWrapper {
                payment,
                status_before: PreviousStatusFlag::NotLocked,
            }
        }
    }

    fn lock_and_refund(
        &self,
        to: &ManagedAddress,
        payment: EsdtTokenPayment<Self::Api>,
        current_epoch: u64,
        unlock_epoch: u64,
    ) {
        if payment.amount == 0 {
            return;
        }

        if unlock_epoch > current_epoch {
            let attributes = LockedTokenAttributes {
                original_token_id: payment.token_identifier,
                original_token_nonce: payment.token_nonce,
                unlock_epoch,
            };

            self.locked_token()
                .nft_create_and_send(to, payment.amount, &attributes);
        } else {
            self.send().direct(
                to,
                &payment.token_identifier,
                payment.token_nonce,
                &payment.amount,
                &[],
            );
        }
    }

    #[proxy]
    fn lp_proxy(&self, sc_address: ManagedAddress) -> lp_proxy::Proxy<Self::Api>;

    #[storage_mapper("lpAddressWhitelist")]
    fn lp_address_whitelist(&self) -> WhitelistMapper<Self::Api, ManagedAddress>;
}
