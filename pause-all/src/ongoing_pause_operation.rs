multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const MIN_GAS_TO_SAVE_PROGRESS: u64 = 10_000_000;
const FIRST_INDEX: usize = 1;

#[derive(TopEncode, TopDecode, Default)]
pub enum OngoingOperation {
    #[default]
    None,
    PauseAll {
        addr_index: usize,
    },
    ResumeAll {
        addr_index: usize,
    },
}

#[multiversx_sc::module]
pub trait OngoingPauseOperationModule:
    multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    fn load_pause_all_operation(&self) -> usize {
        let current_op: OngoingOperation = self.load_operation();
        match current_op {
            OngoingOperation::None => FIRST_INDEX,
            OngoingOperation::PauseAll { addr_index } => addr_index,
            OngoingOperation::ResumeAll { addr_index: _ } => {
                sc_panic!("Resume operation in progress")
            }
        }
    }

    fn load_resume_all_operation(&self) -> usize {
        let current_op: OngoingOperation = self.load_operation();
        match current_op {
            OngoingOperation::None => FIRST_INDEX,
            OngoingOperation::PauseAll { addr_index: _ } => {
                sc_panic!("Pause operation in progress")
            }
            OngoingOperation::ResumeAll { addr_index } => addr_index,
        }
    }
}
