elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct LockedTokenAttributes<M: ManagedTypeApi> {
    pub original_token_id: TokenIdentifier<M>,
    pub original_token_nonce: u64,
    pub unlock_epoch: u64,
}

#[derive(PartialEq, Clone, Copy)]
pub enum PreviousStatusFlag {
    NotLocked,
    Locked { locked_token_nonce: u64 },
}

impl PreviousStatusFlag {
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

#[elrond_wasm::module]
pub trait LockedTokenModule:
    elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
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
        let payment_amount = self.call_value().egld_value();

        self.locked_token().issue(
            EsdtTokenType::Meta,
            payment_amount,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    #[only_owner]
    #[endpoint(setLocalRolesLockedToken)]
    fn set_local_roles_locked_token(&self) {
        self.locked_token().set_local_roles(
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
            ],
            None,
        );
    }

    #[view(getLockedTokenId)]
    #[storage_mapper("lockedTokenId")]
    fn locked_token(&self) -> NonFungibleTokenMapper<Self::Api>;
}
