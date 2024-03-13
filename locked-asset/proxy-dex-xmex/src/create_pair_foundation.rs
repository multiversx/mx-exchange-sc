use common_structs::{Epoch, Nonce, NonceAmountPair, PaymentsVec};
use pausable::ProxyTrait as _;

use crate::{
    create_pair_user::TOKENS_NOT_DEPOSITED_ERR_MSG,
    proxy_interactions::proxy_pair::{AddLiqResultType, RemoveLiqResultType},
};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const INITIAL_LIQ_MIN_VALUE: u32 = 1;

pub static XMEX_NOT_DEPOSITED_ERR_MSG: &[u8] = b"xMex not deposited";
pub static PAIR_NOT_CREATED_ERR_MSG: &[u8] = b"Pair not created";

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct UnlockInfo<M: ManagedTypeApi> {
    pub unlock_epoch: Epoch,
    pub amount: BigUint<M>,
    pub original_depositor_address: ManagedAddress<M>,
}

#[multiversx_sc::module]
pub trait CreatePairFoundationModule:
    crate::create_pair_user::CreatePairUserModule
    + crate::other_sc_whitelist::OtherScWhitelistModule
    + energy_query::EnergyQueryModule
    + crate::proxy_interactions::proxy_pair::ProxyPairModule
    + crate::proxy_interactions::proxy_common::ProxyCommonModule
    + crate::proxy_interactions::pair_interactions::PairInteractionsModule
    + crate::merge_tokens::wrapped_lp_token_merge::WrappedLpTokenMerge
    + crate::energy_update::EnergyUpdateModule
    + crate::events::EventsModule
    + token_merge_helper::TokenMergeHelperModule
    + token_send::TokenSendModule
    + utils::UtilsModule
    + legacy_token_decode_module::LegacyTokenDecodeModule
{
    #[only_owner]
    #[endpoint(setFoundationAddress)]
    fn set_foundation_address(&self, foundation_address: ManagedAddress) {
        self.foundation_address().set(foundation_address);
    }

    #[only_owner]
    #[endpoint(setLpLockEpochs)]
    fn set_lp_lock_epochs(&self, lp_lock_epochs: Epoch) {
        require!(lp_lock_epochs > 0, "Invalid lock epochs");

        self.lp_lock_epochs().set(lp_lock_epochs);
    }

    #[payable("*")]
    #[endpoint(depositXmex)]
    fn deposit_xmex(&self, token_id: TokenIdentifier) {
        self.require_foundation_caller();

        let info_mapper = self.token_info(&token_id);
        require!(!info_mapper.is_empty(), TOKENS_NOT_DEPOSITED_ERR_MSG);

        let token_info = info_mapper.get();
        require!(token_info.opt_pair.is_some(), PAIR_NOT_CREATED_ERR_MSG);

        let xmex_mapper = self.xmex_for_token(&token_id);
        require!(xmex_mapper.is_empty(), "xMex already deposited");

        let payment = self.call_value().single_esdt();
        let locked_mex_token_id = self.get_locked_token_id();
        require!(
            payment.token_identifier == locked_mex_token_id,
            "Invalid payment"
        );

        // TODO: Should this do any validation based on requested price?

        xmex_mapper.set(NonceAmountPair::new(payment.token_nonce, payment.amount));
    }

    #[endpoint(withdrawXmex)]
    fn withdraw_xmex(&self, token_id: TokenIdentifier) {
        self.require_foundation_caller();

        let xmex_mapper = self.xmex_for_token(&token_id);
        require!(!xmex_mapper.is_empty(), XMEX_NOT_DEPOSITED_ERR_MSG);

        let nonce_amount_pair = xmex_mapper.take();
        let caller = self.blockchain().get_caller();
        let xmex_token_id = self.get_locked_token_id();
        self.send().direct_esdt(
            &caller,
            &xmex_token_id,
            nonce_amount_pair.nonce,
            &nonce_amount_pair.amount,
        );
    }

    #[endpoint(addInitialLiquidityFromDeposits)]
    fn add_initial_liq_from_deposits(&self, token_id: TokenIdentifier) {
        self.require_foundation_caller();

        let info_mapper = self.token_info(&token_id);
        require!(!info_mapper.is_empty(), TOKENS_NOT_DEPOSITED_ERR_MSG);

        let xmex_mapper = self.xmex_for_token(&token_id);
        require!(!xmex_mapper.is_empty(), XMEX_NOT_DEPOSITED_ERR_MSG);

        let token_info = info_mapper.take();
        require!(token_info.opt_pair.is_some(), PAIR_NOT_CREATED_ERR_MSG);

        let user_tokens_payment = EsdtTokenPayment::new(token_id, 0, token_info.deposited_tokens);

        let xmex_token_id = self.get_locked_token_id();
        let xmex_nonce_amount_pair = xmex_mapper.take();
        let xmex_payment = EsdtTokenPayment::new(
            xmex_token_id,
            xmex_nonce_amount_pair.nonce,
            xmex_nonce_amount_pair.amount,
        );

        let pair_addr = unsafe { token_info.opt_pair.unwrap_unchecked() };
        let add_liq_result = self.add_initial_liq(pair_addr, user_tokens_payment, xmex_payment);
        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &add_liq_result.locked_token_leftover);
        self.send_payment_non_zero(&token_info.depositor, &add_liq_result.other_token_leftover);

        let current_epoch = self.blockchain().get_block_epoch();
        let lock_epochs = self.lp_lock_epochs().get();
        self.lp_unlock_info(add_liq_result.new_wrapped_token.token_nonce)
            .set(UnlockInfo {
                unlock_epoch: current_epoch + lock_epochs,
                amount: add_liq_result.new_wrapped_token.amount,
                original_depositor_address: token_info.depositor,
            });
    }

    #[endpoint(removeLiqCreatedPair)]
    fn remove_liq_created_pair(
        &self,
        pair_address: ManagedAddress,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
        wrapped_token_nonce: Nonce,
    ) -> RemoveLiqResultType<Self::Api> {
        self.require_foundation_caller();
        self.require_is_intermediated_pair(&pair_address);
        self.require_wrapped_lp_token_id_not_empty();

        let lp_unlock_info_mapper = self.lp_unlock_info(wrapped_token_nonce);
        require!(!lp_unlock_info_mapper.is_empty(), "Token does not exist");

        let current_epoch = self.blockchain().get_block_epoch();
        let unlock_info = lp_unlock_info_mapper.take();
        require!(
            current_epoch >= unlock_info.unlock_epoch,
            "May not unlock yet"
        );

        let wrapped_lp_token_id = self.wrapped_lp_token().get_token_id();
        let wrapped_lp_payment =
            EsdtTokenPayment::new(wrapped_lp_token_id, wrapped_token_nonce, unlock_info.amount);
        let remove_liq_result = self.remove_liquidity_proxy_common(
            wrapped_lp_payment,
            pair_address,
            first_token_amount_min,
            second_token_amount_min,
        );

        let caller = self.blockchain().get_caller();
        self.send_payment_non_zero(&caller, &remove_liq_result.locked_tokens);
        self.send_payment_non_zero(
            &unlock_info.original_depositor_address,
            &remove_liq_result.other_tokens,
        );

        if let Some(unlocked_tokens) = &remove_liq_result.opt_unlocked_tokens {
            self.send_payment_non_zero(&caller, unlocked_tokens);
        }

        remove_liq_result
    }

    fn add_initial_liq(
        &self,
        pair_address: ManagedAddress,
        user_custom_tokens: EsdtTokenPayment,
        xmex_tokens: EsdtTokenPayment,
    ) -> AddLiqResultType<Self::Api> {
        let _: IgnoreValue = self
            .pair_proxy(pair_address.clone())
            .resume()
            .execute_on_dest_context();

        let mut payments = PaymentsVec::from_single_item(user_custom_tokens);
        payments.push(xmex_tokens);

        self.add_liquidity_proxy(
            pair_address,
            BigUint::from(INITIAL_LIQ_MIN_VALUE),
            BigUint::from(INITIAL_LIQ_MIN_VALUE),
            payments,
        )
    }

    fn require_foundation_caller(&self) {
        let caller = self.blockchain().get_caller();
        let foundation_address = self.foundation_address().get();
        require!(caller == foundation_address, "Invalid caller");
    }

    #[proxy]
    fn pair_proxy(&self, sc_address: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[view(getFoundationAddress)]
    #[storage_mapper("foundationAddress")]
    fn foundation_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("xMexForToken")]
    fn xmex_for_token(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<NonceAmountPair<Self::Api>>;

    #[storage_mapper("lpLockEpochs")]
    fn lp_lock_epochs(&self) -> SingleValueMapper<Epoch>;

    #[view(getLpUnlockInfo)]
    #[storage_mapper("lpUnlockInfo")]
    fn lp_unlock_info(
        &self,
        wrapped_token_nonce: Nonce,
    ) -> SingleValueMapper<UnlockInfo<Self::Api>>;
}
