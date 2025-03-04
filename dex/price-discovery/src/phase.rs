use crate::Timestamp;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, PartialEq, PartialOrd)]
pub enum Phase {
    Idle,
    UserDepositWithdraw,
    OwnerDepositWithdraw,
    Redeem,
}

#[multiversx_sc::module]
pub trait PhaseModule:
    crate::common_storage::CommonStorageModule + crate::events::EventsModule
{
    #[view(getCurrentPhase)]
    fn get_current_phase(&self) -> Phase {
        let current_time = self.blockchain().get_block_timestamp();
        let start_time = self.start_time().get();
        if current_time < start_time {
            return Phase::Idle;
        }

        let user_deposit_time = self.user_deposit_withdraw_time().get();
        let user_deposit_phase_end = start_time + user_deposit_time;
        if current_time < user_deposit_phase_end {
            return Phase::UserDepositWithdraw;
        }

        let owner_deposit_time = self.owner_deposit_withdraw_time().get();
        let owner_deposit_phase_end = user_deposit_phase_end + owner_deposit_time;
        if current_time < owner_deposit_phase_end {
            return Phase::OwnerDepositWithdraw;
        }

        Phase::Redeem
    }

    fn require_user_deposit_withdraw_allowed(&self, phase: &Phase) {
        require!(
            phase == &Phase::UserDepositWithdraw,
            "User deposit/withdraw not allowed in this phase"
        );
    }

    fn require_owner_deposit_withdraw_allowed(&self, phase: &Phase) {
        require!(
            phase == &Phase::OwnerDepositWithdraw,
            "Owner deposit/withdraw not allowed in this phase"
        );
    }

    fn require_redeem_allowed(&self, phase: &Phase) {
        require!(phase == &Phase::Redeem, "Redeem not allowed in this phase");
    }

    #[view(getUserDepositWithdrawTime)]
    #[storage_mapper("userDepositWithdrawTime")]
    fn user_deposit_withdraw_time(&self) -> SingleValueMapper<Timestamp>;

    #[view(getOwnerDepositWithdrawTime)]
    #[storage_mapper("ownerDepositWithdrawTime")]
    fn owner_deposit_withdraw_time(&self) -> SingleValueMapper<Timestamp>;
}
