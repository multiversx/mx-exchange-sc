elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::{create_pool, events};

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, PartialEq)]
pub enum Phase<M: ManagedTypeApi> {
    Idle,
    NoPenalty,
    LinearIncreasingPenalty { penalty_percentage: BigUint<M> },
    OnlyWithdrawFixedPenalty { penalty_percentage: BigUint<M> },
    Unbond,
    Redeem,
}

impl<M: ManagedTypeApi> Phase<M> {
    pub fn get_penalty_percentage(&self) -> BigUint<M> {
        match self {
            Self::LinearIncreasingPenalty { penalty_percentage } => penalty_percentage.clone(),
            Self::OnlyWithdrawFixedPenalty { penalty_percentage } => penalty_percentage.clone(),
            _ => BigUint::zero(),
        }
    }
}

#[elrond_wasm::module]
pub trait PhaseModule:
    crate::common_storage::CommonStorageModule + create_pool::CreatePoolModule + events::EventsModule
{
    #[view(getCurrentPhase)]
    fn get_current_phase(&self) -> Phase<Self::Api> {
        let current_block = self.blockchain().get_block_nonce();
        let start_block = self.start_block().get();
        if current_block < start_block {
            return Phase::Idle;
        }

        let no_limit_phase_duration_blocks = self.no_limit_phase_duration_blocks().get();
        let no_limit_phase_end = start_block + no_limit_phase_duration_blocks;
        if current_block < no_limit_phase_end {
            return Phase::NoPenalty;
        }

        let linear_penalty_phase_duration_blocks =
            self.linear_penalty_phase_duration_blocks().get();
        let linear_penalty_phase_start = no_limit_phase_end;
        let linear_penalty_phase_end =
            linear_penalty_phase_start + linear_penalty_phase_duration_blocks;
        if current_block < linear_penalty_phase_end {
            let blocks_passed_in_penalty_phase = current_block - linear_penalty_phase_start;
            let min_percentage = self.penalty_min_percentage().get();
            let max_percentage = self.penalty_max_percentage().get();
            let percentage_diff = &max_percentage - &min_percentage;

            let penalty_percentage_increase = if linear_penalty_phase_duration_blocks > 1 {
                percentage_diff * blocks_passed_in_penalty_phase
                    / (linear_penalty_phase_duration_blocks - 1)
            } else {
                BigUint::zero()
            };

            return Phase::LinearIncreasingPenalty {
                penalty_percentage: min_percentage + penalty_percentage_increase,
            };
        }

        let fixed_penalty_phase_duration_blocks = self.fixed_penalty_phase_duration_blocks().get();
        let fixed_penalty_phase_start = linear_penalty_phase_end;
        let fixed_penalty_phase_end =
            fixed_penalty_phase_start + fixed_penalty_phase_duration_blocks;
        if current_block < fixed_penalty_phase_end {
            return Phase::OnlyWithdrawFixedPenalty {
                penalty_percentage: self.fixed_penalty_percentage().get(),
            };
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let pool_creation_epoch = self.pool_creation_epoch().get();
        let unbond_period_epochs = self.unbond_period_epochs().get();
        let redeem_epoch = pool_creation_epoch + unbond_period_epochs;

        if current_epoch < redeem_epoch {
            return Phase::Unbond;
        }

        Phase::Redeem
    }

    fn require_deposit_allowed(&self, phase: &Phase<Self::Api>) {
        match phase {
            Phase::Idle
            | Phase::OnlyWithdrawFixedPenalty {
                penalty_percentage: _,
            }
            | Phase::Unbond
            | Phase::Redeem => {
                sc_panic!("Deposit not allowed in this phase")
            }
            _ => {}
        };
    }

    fn require_withdraw_allowed(&self, phase: &Phase<Self::Api>) {
        match phase {
            Phase::Idle | Phase::Unbond | Phase::Redeem => {
                sc_panic!("Withdraw not allowed in this phase")
            }
            _ => {}
        };
    }

    fn require_deposit_extra_rewards_allowed(&self, phase: &Phase<Self::Api>) {
        match phase {
            Phase::Unbond | Phase::Redeem => {
                sc_panic!("Deposit extra rewards not allowed in this phase")
            }
            _ => {}
        };
    }

    fn require_redeem_allowed(&self, phase: &Phase<Self::Api>) {
        let pool_creation_epoch = self.pool_creation_epoch().get();
        require!(pool_creation_epoch > 0, "Liquidity Pool not created yet");

        require!(phase == &Phase::Redeem, "Unbond period not finished yet");
    }

    #[view(getNoLimitPhaseDurationBlocks)]
    #[storage_mapper("noLimitPhaseDurationBlocks")]
    fn no_limit_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getLinearPenaltyPhaseDurationBlocks)]
    #[storage_mapper("linearPenaltyPhaseDurationBlocks")]
    fn linear_penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getFixedPenaltyPhaseDurationBlocks)]
    #[storage_mapper("fixedPenaltyPhaseDurationBlocks")]
    fn fixed_penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getPenaltyMinPercentage)]
    #[storage_mapper("penaltyMinPercentage")]
    fn penalty_min_percentage(&self) -> SingleValueMapper<BigUint>;

    #[view(getPenaltyMaxPercentage)]
    #[storage_mapper("penaltyMaxPercentage")]
    fn penalty_max_percentage(&self) -> SingleValueMapper<BigUint>;

    #[view(getFixedPenaltyPercentage)]
    #[storage_mapper("fixedPenaltyPercentage")]
    fn fixed_penalty_percentage(&self) -> SingleValueMapper<BigUint>;
}
