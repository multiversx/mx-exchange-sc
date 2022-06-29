#![no_std]

use ongoing_pause_operation::{OngoingOperation, CONTINUE_OP, STOP_OP};

elrond_wasm::imports!();

mod pause_proxy {
    elrond_wasm::imports!();

    #[elrond_wasm::proxy]
    pub trait Pausable {
        #[endpoint]
        fn pause(&self);

        #[endpoint]
        fn resume(&self);
    }
}

pub mod ongoing_pause_operation;

#[elrond_wasm::contract]
pub trait PauseAll: ongoing_pause_operation::OngoingPauseOperationModule {
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

    #[only_owner]
    #[endpoint(pauseAll)]
    fn pause_all(&self) -> OperationCompletionStatus {
        let mut current_index = self.load_pause_all_operation();
        let whitelist = self.pausable_contracts();
        let whitelist_len = whitelist.len();

        let run_result = self.run_while_it_has_gas(|| {
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
        self.pause_proxy(sc_addr)
            .pause()
            .execute_on_dest_context_ignore_result();
    }

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

    #[only_owner]
    #[endpoint(resumeAll)]
    fn resume_all(&self) -> OperationCompletionStatus {
        let mut current_index = self.load_resume_all_operation();
        let whitelist = self.pausable_contracts();
        let whitelist_len = whitelist.len();

        let run_result = self.run_while_it_has_gas(|| {
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
        self.pause_proxy(sc_addr)
            .resume()
            .execute_on_dest_context_ignore_result();
    }

    #[proxy]
    fn pause_proxy(&self, addr: ManagedAddress) -> pause_proxy::Proxy<Self::Api>;

    #[view(getPausableContracts)]
    #[storage_mapper("pausableContracts")]
    fn pausable_contracts(&self) -> UnorderedSetMapper<ManagedAddress>;
}
