elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct LockedTokenAttributes<M: ManagedTypeApi> {
    pub original_token_id: TokenIdentifier<M>,
    pub original_token_nonce: u64,
    pub unlock_epoch: u64,
}

#[derive(PartialEq)]
pub enum PreviousStatusFlag {
    NotLocked,
    Locked { unlock_epoch: u64 },
}

impl PreviousStatusFlag {
    #[inline]
    pub fn was_locked(&self) -> bool {
        matches!(*self, PreviousStatusFlag::Locked { unlock_epoch: _ })
    }

    pub fn get_unlock_epoch(&self) -> u64 {
        match *self {
            PreviousStatusFlag::NotLocked => 0,
            PreviousStatusFlag::Locked { unlock_epoch } => unlock_epoch,
        }
    }
}

pub struct UnlockedPaymentWrapper<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub status_before: PreviousStatusFlag,
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
