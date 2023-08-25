multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedDecode, NestedEncode, PartialEq, Debug, Clone)]
pub struct LockedTokenAttributes<M: ManagedTypeApi> {
    pub original_token_id: EgldOrEsdtTokenIdentifier<M>,
    pub original_token_nonce: u64,
    pub unlock_epoch: u64,
}

#[derive(PartialEq, Clone, Copy)]
pub enum PreviousStatusFlag {
    NotLocked,
    Locked { locked_token_nonce: u64 },
}

impl PreviousStatusFlag {
    pub fn new(locked_token_nonce: u64) -> Self {
        if locked_token_nonce == 0 {
            PreviousStatusFlag::NotLocked
        } else {
            PreviousStatusFlag::Locked { locked_token_nonce }
        }
    }

    #[inline]
    pub fn was_locked(&self) -> bool {
        matches!(
            *self,
            PreviousStatusFlag::Locked {
                locked_token_nonce: _
            }
        )
    }
}

pub struct UnlockedPaymentWrapper<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub status_before: PreviousStatusFlag,
}

impl<M: ManagedTypeApi> UnlockedPaymentWrapper<M> {
    pub fn get_locked_token_nonce(&self) -> u64 {
        match self.status_before {
            PreviousStatusFlag::NotLocked => 0,
            PreviousStatusFlag::Locked { locked_token_nonce } => locked_token_nonce,
        }
    }
}

#[multiversx_sc::module]
pub trait LockedTokenModule:
    crate::token_attributes::TokenAttributesModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueLockedToken)]
    fn issue_locked_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let payment_amount = self.call_value().egld_value().clone_value();

        self.locked_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    fn send_tokens_optimal_status(
        &self,
        to: &ManagedAddress,
        payment: EsdtTokenPayment<Self::Api>,
        prev_status: PreviousStatusFlag,
    ) -> EsdtTokenPayment<Self::Api> {
        if payment.amount == 0 {
            return payment;
        }

        if let PreviousStatusFlag::Locked { locked_token_nonce } = prev_status {
            let locked_token_mapper = self.locked_token();
            let attributes: LockedTokenAttributes<Self::Api> =
                locked_token_mapper.get_token_attributes(locked_token_nonce);

            let current_epoch = self.blockchain().get_block_epoch();
            if current_epoch < attributes.unlock_epoch {
                let locked_token_nonce = self.get_or_create_nonce_for_attributes(
                    &locked_token_mapper,
                    payment.token_identifier.as_managed_buffer(),
                    &attributes,
                );

                return locked_token_mapper.nft_add_quantity_and_send(
                    to,
                    locked_token_nonce,
                    payment.amount,
                );
            }
        }

        self.send().direct_esdt(
            to,
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        payment
    }

    #[view(getLockedTokenId)]
    #[storage_mapper("lockedTokenId")]
    fn locked_token(&self) -> NonFungibleTokenMapper;
}
