elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const MAX_PERCENTAGE: u64 = 10_000; // 100%

#[derive(TypeAbi, TopEncode, TopDecode)]
pub enum Phase<M: ManagedTypeApi> {
    Idle,
    NoPenalty,
    LinearIncreasingPenalty {
        current_penalty_percentage: BigUint<M>,
    },
    Unbond,
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

        let penalty_phase_duration_blocks = self.penalty_phase_duration_blocks().get();
        let penalty_phase_start = no_limit_phase_end;
        let penalty_phase_end = penalty_phase_start + penalty_phase_duration_blocks;
        if current_block < penalty_phase_end {
            let blocks_passed_in_penalty_phase = current_block - penalty_phase_start;
            let min_percentage = self.penalty_min_percentage().get();
            let max_percentage = self.penalty_max_percentage().get();
            let percentage_diff = &max_percentage - &min_percentage;

            // TODO: Think about precision
            let penalty_percentage_increase =
                percentage_diff * blocks_passed_in_penalty_phase / penalty_phase_duration_blocks;

            return Phase::LinearIncreasingPenalty {
                current_penalty_percentage: min_percentage + penalty_percentage_increase,
            };
        }

        Phase::Unbond
    }

    #[storage_mapper("noLimitPhaseDurationBocks")]
    fn no_limit_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("penaltyPhaseDurationBlocks")]
    fn penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("unbondPeriodEpochs")]
    fn unbond_period_epochs(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("penaltyMinPercentage")]
    fn penalty_min_percentage(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("penaltyMaxPercentage")]
    fn penalty_max_percentage(&self) -> SingleValueMapper<BigUint>;
}
