elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq)]
pub enum Phase<M: ManagedTypeApi> {
    Idle,
    NoPenalty,
    LinearIncreasingPenalty { penalty_percentage: BigUint<M> },
    OnlyWithdrawFixedPenalty { penalty_percentage: BigUint<M> },
    Unbond,
}

impl<M: ManagedTypeApi> Phase<M> {
    pub fn to_penalty_percentage(self) -> BigUint<M> {
        match self {
            Self::LinearIncreasingPenalty { penalty_percentage } => penalty_percentage,
            Self::OnlyWithdrawFixedPenalty { penalty_percentage } => penalty_percentage,
            _ => BigUint::zero(),
        }
    }
}

#[elrond_wasm::module]
pub trait PhaseModule: crate::common_storage::CommonStorageModule {
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

        Phase::Unbond
    }

    fn require_deposit_allowed(&self, phase: &Phase<Self::Api>) {
        match phase {
            Phase::Idle
            | Phase::OnlyWithdrawFixedPenalty {
                penalty_percentage: _,
            }
            | Phase::Unbond => {
                sc_panic!("Deposit not allowed in this phase")
            }
            _ => {}
        };
    }

    fn require_withdraw_allowed(&self, phase: &Phase<Self::Api>) {
        match phase {
            Phase::Idle | Phase::Unbond => sc_panic!("Withdraw not allowed in this phase"),
            _ => {}
        };
    }

    #[storage_mapper("noLimitPhaseDurationBocks")]
    fn no_limit_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("linearPenaltyPhaseDurationBlocks")]
    fn linear_penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("fixedPenaltyPhaseDurationBlocks")]
    fn fixed_penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("unbondPeriodEpochs")]
    fn unbond_period_epochs(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("penaltyMinPercentage")]
    fn penalty_min_percentage(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("penaltyMaxPercentage")]
    fn penalty_max_percentage(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("fixedPenaltyPercentage")]
    fn fixed_penalty_percentage(&self) -> SingleValueMapper<BigUint>;
}
