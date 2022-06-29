elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const MIN_GAS_TO_SAVE_PROGRESS: u64 = 10_000_000;
const FIRST_INDEX: usize = 1;

#[derive(TopEncode, TopDecode)]
pub enum OngoingOperation {
    None,
    PauseAll { addr_index: usize },
    ResumeAll { addr_index: usize },
}

pub type LoopOp = bool;
pub const CONTINUE_OP: bool = true;
pub const STOP_OP: bool = false;

#[elrond_wasm::module]
pub trait OngoingPauseOperationModule {
    fn run_while_it_has_gas<Process>(&self, mut process: Process) -> OperationCompletionStatus
    where
        Process: FnMut() -> LoopOp,
    {
        let mut gas_per_iteration = 0;
        let mut gas_before = self.blockchain().get_gas_left();
        loop {
            let loop_op = process();
            if loop_op == STOP_OP {
                break;
            }

            let gas_after = self.blockchain().get_gas_left();
            let current_iteration_cost = gas_before - gas_after;
            if current_iteration_cost > gas_per_iteration {
                gas_per_iteration = current_iteration_cost;
            }

            if !self.can_continue_operation(gas_per_iteration) {
                return OperationCompletionStatus::InterruptedBeforeOutOfGas;
            }

            gas_before = gas_after;
        }

        self.clear_operation();

        OperationCompletionStatus::Completed
    }

    fn can_continue_operation(&self, operation_cost: u64) -> bool {
        let gas_left = self.blockchain().get_gas_left();

        gas_left > MIN_GAS_TO_SAVE_PROGRESS + operation_cost
    }

    #[inline]
    fn save_progress(&self, op: &OngoingOperation) {
        self.current_ongoing_operation().set(op);
    }

    #[inline]
    fn clear_operation(&self) {
        self.current_ongoing_operation().clear();
    }

    fn load_pause_all_operation(&self) -> usize {
        let current_op = self.current_ongoing_operation().get();
        match current_op {
            OngoingOperation::None => FIRST_INDEX,
            OngoingOperation::PauseAll { addr_index } => addr_index,
            OngoingOperation::ResumeAll { addr_index: _ } => {
                sc_panic!("Resume operation in progress")
            }
        }
    }

    fn load_resume_all_operation(&self) -> usize {
        let current_op = self.current_ongoing_operation().get();
        match current_op {
            OngoingOperation::None => FIRST_INDEX,
            OngoingOperation::PauseAll { addr_index: _ } => {
                sc_panic!("Pause operation in progress")
            }
            OngoingOperation::ResumeAll { addr_index } => addr_index,
        }
    }

    #[storage_mapper("operation")]
    fn current_ongoing_operation(&self) -> SingleValueMapper<OngoingOperation>;
}
