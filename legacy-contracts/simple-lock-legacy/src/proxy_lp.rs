multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::{error_messages::CANNOT_UNLOCK_YET_ERR_MSG, locked_token::LockedTokenAttributes};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct LpProxyTokenAttributes<M: ManagedTypeApi> {
    pub lp_token_id: TokenIdentifier<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub first_token_locked_nonce: u64,
    pub second_token_id: TokenIdentifier<M>,
    pub second_token_locked_nonce: u64,
}

#[multiversx_sc::module]
pub trait ProxyLpModule: crate::locked_token::LockedTokenModule {
    #[payable("*")]
    #[endpoint(removeLiquidityLockedToken)]
    fn remove_liquidity_locked_token(&self) -> EsdtTokenPayment {
        let payment = self.call_value().single_esdt();
        let caller = self.blockchain().get_caller();

        let unlocked_lp_token_id =
            self.check_and_get_unlocked_lp_token(&payment.token_identifier, payment.token_nonce);

        self.send().esdt_local_burn(
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        let output_token_payment = EsdtTokenPayment::new(unlocked_lp_token_id, 0, payment.amount);

        self.send().direct_esdt(
            &caller,
            &output_token_payment.token_identifier,
            output_token_payment.token_nonce,
            &output_token_payment.amount,
        );

        output_token_payment
    }

    fn check_and_get_unlocked_lp_token(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> TokenIdentifier {
        let lp_proxy_token_mapper: NonFungibleTokenMapper<Self::Api> = self.lp_proxy_token();
        lp_proxy_token_mapper.require_same_token(token_id);

        let lp_proxy_token_attributes: LpProxyTokenAttributes<Self::Api> =
            lp_proxy_token_mapper.get_token_attributes(token_nonce);

        let current_epoch = self.blockchain().get_block_epoch();
        if lp_proxy_token_attributes.first_token_locked_nonce > 0 {
            let token_attributes: LockedTokenAttributes<Self::Api> = self
                .locked_token()
                .get_token_attributes(lp_proxy_token_attributes.first_token_locked_nonce);

            require!(
                token_attributes.unlock_epoch >= current_epoch,
                CANNOT_UNLOCK_YET_ERR_MSG
            );
        }
        if lp_proxy_token_attributes.second_token_locked_nonce > 0 {
            let token_attributes: LockedTokenAttributes<Self::Api> = self
                .locked_token()
                .get_token_attributes(lp_proxy_token_attributes.second_token_locked_nonce);

            require!(
                token_attributes.unlock_epoch >= current_epoch,
                CANNOT_UNLOCK_YET_ERR_MSG
            );
        }

        lp_proxy_token_attributes.lp_token_id
    }

    #[view(getLpProxyTokenId)]
    #[storage_mapper("lpProxyTokenId")]
    fn lp_proxy_token(&self) -> NonFungibleTokenMapper;
}
