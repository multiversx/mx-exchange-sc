#![no_std]

use multiversx_sc_modules::ongoing_operation::{CONTINUE_OP, STOP_OP};
use ongoing_pause_operation::{OngoingOperation, MIN_GAS_TO_SAVE_PROGRESS};

multiversx_sc::imports!();

mod pause_proxy {
    multiversx_sc::imports!();

    #[multiversx_sc::proxy]
    pub trait Pausable {
        #[endpoint]
        fn pause(&self);

        #[endpoint]
        fn resume(&self);
    }
}

pub mod ongoing_pause_operation;

#[multiversx_sc::contract]
pub trait PauseAll:
    ongoing_pause_operation::OngoingPauseOperationModule
    + multiversx_sc_modules::ongoing_operation::OngoingOperationModule
{
    #[init]
    fn init(&self) {}

    #[only_owner]
    #[endpoint(addPausableContracts)]
    fn add_pausable_contracts(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let mut whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            let _ = whitelist.insert(addr);
        }
    }

    #[only_owner]
    #[endpoint(removePausableContracts)]
    fn remove_pausable_contracts(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let mut whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            let _ = whitelist.swap_remove(&addr);
        }
    }

    /// Will pause the given list of contracts.
    /// Contracts will only be paused if they are in the pausable_contracts list.
    /// Other contracts will be ignored.
    #[only_owner]
    #[endpoint(pauseSelected)]
    fn pause_selected(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            if whitelist.contains(&addr) {
                self.call_pause(addr);
            }
        }
    }

    /// Will attempt to pause all contracts from the whitelist.
    /// Returns "completed" if all were paused.
    /// Otherwise, it will save progress and return "interrupted",
    /// and will require more calls to complete
    #[only_owner]
    #[endpoint(pauseAll)]
    fn pause_all(&self) -> OperationCompletionStatus {
        let mut current_index = self.load_pause_all_operation();
        let whitelist = self.pausable_contracts();
        let whitelist_len = whitelist.len();

        let run_result = self.run_while_it_has_gas(MIN_GAS_TO_SAVE_PROGRESS, || {
            if current_index > whitelist_len {
                return STOP_OP;
            }

            let sc_addr = whitelist.get_by_index(current_index);
            self.call_pause(sc_addr);
            current_index += 1;

            CONTINUE_OP
        });
        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&OngoingOperation::PauseAll {
                addr_index: current_index,
            });
        }

        run_result
    }

    fn call_pause(&self, sc_addr: ManagedAddress) {
        let _: IgnoreValue = self.pause_proxy(sc_addr).pause().execute_on_dest_context();
    }

    /// Will unpause the given list of contracts.
    /// Contracts not in the whitelist will be ignored.
    #[only_owner]
    #[endpoint(resumeSelected)]
    fn resume_selected(&self, pausable_sc_addr: MultiValueEncoded<ManagedAddress>) {
        let whitelist = self.pausable_contracts();
        for addr in pausable_sc_addr {
            if whitelist.contains(&addr) {
                self.call_resume(addr);
            }
        }
    }

    /// Will attempt to unpause all contracts from the whitelist.
    /// Returns "completed" if all were unpaused.
    /// Otherwise, it will save progress and return "interrupted",
    /// and will require more calls to complete
    #[only_owner]
    #[endpoint(resumeAll)]
    fn resume_all(&self) -> OperationCompletionStatus {
        let mut current_index = self.load_resume_all_operation();
        let whitelist = self.pausable_contracts();
        let whitelist_len = whitelist.len();

        let run_result = self.run_while_it_has_gas(MIN_GAS_TO_SAVE_PROGRESS, || {
            if current_index > whitelist_len {
                return STOP_OP;
            }

            let sc_addr = whitelist.get_by_index(current_index);
            self.call_resume(sc_addr);
            current_index += 1;

            CONTINUE_OP
        });
        if run_result == OperationCompletionStatus::InterruptedBeforeOutOfGas {
            self.save_progress(&OngoingOperation::ResumeAll {
                addr_index: current_index,
            });
        }

        run_result
    }

    fn call_resume(&self, sc_addr: ManagedAddress) {
        let _: IgnoreValue = self.pause_proxy(sc_addr).resume().execute_on_dest_context();
    }

    #[proxy]
    fn pause_proxy(&self, addr: ManagedAddress) -> pause_proxy::Proxy<Self::Api>;

    #[view(getPausableContracts)]
    #[storage_mapper("pausableContracts")]
    fn pausable_contracts(&self) -> UnorderedSetMapper<ManagedAddress>;
}
