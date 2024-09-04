use crate::{
    action::{Action, ActionFullInfo, GasLimit},
    multisig_state::{ActionId, ActionStatus, GroupId},
    user_role::UserRole,
};

use multiversx_sc::imports::*;

/// Gas required to finish transaction after transfer-execute.
const PERFORM_ACTION_FINISH_GAS: u64 = 300_000;
pub const MAX_BOARD_MEMBERS: usize = 30;

fn usize_add_isize(value: &mut usize, delta: isize) {
    *value = (*value as isize + delta) as usize;
}

/// Contains all events that can be emitted by the contract.
#[multiversx_sc::module]
pub trait MultisigPerformModule:
    crate::multisig_state::MultisigStateModule + crate::multisig_events::MultisigEventsModule
{
    fn ensure_and_get_gas_for_transfer_exec(&self) -> GasLimit {
        let gas_left = self.blockchain().get_gas_left();
        require!(
            gas_left > PERFORM_ACTION_FINISH_GAS,
            "insufficient gas for call"
        );
        gas_left - PERFORM_ACTION_FINISH_GAS
    }

    /// Can be used to:
    /// - create new user (board member / proposer)
    /// - remove user (board member / proposer)
    /// - reactivate removed user
    /// - convert between board member and proposer
    /// Will keep the board size and proposer count in sync.
    fn change_user_role(
        &self,
        action_id: ActionId,
        user_address: ManagedAddress,
        new_role: UserRole,
    ) {
        let user_id = if new_role == UserRole::None {
            // avoid creating a new user just to delete it
            let user_id = self.user_mapper().get_user_id(&user_address);
            if user_id == 0 {
                return;
            }
            user_id
        } else {
            self.user_mapper().get_or_create_user(&user_address)
        };

        let user_id_to_role_mapper = self.user_id_to_role(user_id);
        let old_role = user_id_to_role_mapper.get();
        user_id_to_role_mapper.set(new_role);

        self.perform_change_user_event(action_id, &user_address, old_role, new_role);

        // update board size
        let mut board_members_delta = 0isize;
        if old_role == UserRole::BoardMember {
            board_members_delta -= 1;
        }
        if new_role == UserRole::BoardMember {
            board_members_delta += 1;
        }
        if board_members_delta != 0 {
            self.num_board_members()
                .update(|value| usize_add_isize(value, board_members_delta));
        }

        let mut proposers_delta = 0isize;
        if old_role == UserRole::Proposer {
            proposers_delta -= 1;
        }
        if new_role == UserRole::Proposer {
            proposers_delta += 1;
        }
        if proposers_delta != 0 {
            self.num_proposers()
                .update(|value| usize_add_isize(value, proposers_delta));
        }
    }

    /// Returns `true` (`1`) if `getActionValidSignerCount >= getQuorum`.
    #[view(quorumReached)]
    fn quorum_reached(&self, action_id: ActionId) -> bool {
        let quorum = self.quorum_for_action(action_id).get();
        let valid_signers_count = self.get_action_valid_signer_count(action_id);
        valid_signers_count >= quorum
    }

    fn clear_action(&self, action_id: ActionId) {
        self.action_mapper().clear_entry_unchecked(action_id);
        self.action_signer_ids(action_id).clear();

        let group_id = self.group_for_action(action_id).take();
        if group_id != 0 {
            let _ = self.action_groups(group_id).swap_remove(&action_id);
        }
    }

    /// Proposers and board members use this to launch signed actions.
    #[endpoint(performAction)]
    fn perform_action_endpoint(&self, action_id: ActionId) -> OptionalValue<ManagedAddress> {
        let (_, caller_role) = self.get_caller_id_and_role();
        require!(
            caller_role.can_perform_action(),
            "only board members and proposers can perform actions"
        );
        require!(
            self.quorum_reached(action_id),
            "quorum has not been reached"
        );

        let group_id = self.group_for_action(action_id).get();
        require!(group_id == 0, "May not execute this action by itself");

        self.perform_action(action_id)
    }

    fn try_perform_action(&self, action_id: ActionId) -> OptionalValue<ManagedAddress> {
        let (_, caller_role) = self.get_caller_id_and_role();
        require!(
            caller_role.can_perform_action(),
            "only board members and proposers can perform actions"
        );
        if self.quorum_reached(action_id) {
            let group_id = self.group_for_action(action_id).get();
            require!(group_id == 0, "May not execute this action by itself");

            return self.perform_action(action_id);
        }
        OptionalValue::None
    }

    /// Perform all the actions in the given batch
    #[endpoint(performBatch)]
    fn perform_batch(&self, group_id: GroupId) {
        let (_, caller_role) = self.get_caller_id_and_role();
        require!(
            caller_role.can_perform_action(),
            "only board members and proposers can perform actions"
        );

        let group_status = self.action_group_status(group_id).get();
        require!(
            group_status == ActionStatus::Available,
            "cannot perform actions of an aborted batch"
        );

        let mapper = self.action_groups(group_id);
        require!(!mapper.is_empty(), "Invalid group ID");

        let mut action_ids = ManagedVec::<Self::Api, _>::new();
        for action_id in mapper.iter() {
            action_ids.push(action_id);
        }

        for action_id in &action_ids {
            require!(
                self.quorum_reached(action_id),
                "quorum has not been reached"
            );

            let _ = self.perform_action(action_id);
        }
    }

    fn perform_action(&self, action_id: ActionId) -> OptionalValue<ManagedAddress> {
        let action = self.action_mapper().get(action_id);

        let group_id = self.group_for_action(action_id).get();
        if group_id != 0 {
            let group_status = self.action_group_status(group_id).get();
            require!(
                group_status == ActionStatus::Available,
                "cannot perform actions of an aborted batch"
            );
        }
        self.start_perform_action_event(&ActionFullInfo {
            action_id,
            action_data: action.clone(),
            signers: self.get_action_signers(action_id),
            group_id,
        });

        // clean up storage
        // happens before actual execution, because the match provides the return on each branch
        // syntax aside, the async_call_raw kills contract execution so cleanup cannot happen afterwards
        self.clear_action(action_id);

        match action {
            Action::Nothing => OptionalValue::None,
            Action::AddBoardMember(board_member_address) => {
                require!(
                    self.num_board_members().get() < MAX_BOARD_MEMBERS,
                    "board size cannot exceed limit"
                );
                self.change_user_role(action_id, board_member_address, UserRole::BoardMember);

                OptionalValue::None
            }
            Action::AddProposer(proposer_address) => {
                self.change_user_role(action_id, proposer_address, UserRole::Proposer);

                // validation required for the scenario when a board member becomes a proposer
                require!(
                    self.quorum().get() <= self.num_board_members().get(),
                    "quorum cannot exceed board size"
                );
                OptionalValue::None
            }
            Action::RemoveUser(user_address) => {
                self.change_user_role(action_id, user_address, UserRole::None);
                let num_board_members = self.num_board_members().get();
                let num_proposers = self.num_proposers().get();
                require!(
                    num_board_members + num_proposers > 0,
                    "cannot remove all board members and proposers"
                );
                require!(
                    self.quorum().get() <= num_board_members,
                    "quorum cannot exceed board size"
                );
                OptionalValue::None
            }
            Action::ChangeQuorum(new_quorum) => {
                require!(
                    new_quorum <= self.num_board_members().get(),
                    "quorum cannot exceed board size"
                );
                self.quorum().set(new_quorum);
                self.perform_change_quorum_event(action_id, new_quorum);
                OptionalValue::None
            }
            Action::SendTransferExecuteEgld(call_data) => {
                let gas = call_data
                    .opt_gas_limit
                    .unwrap_or_else(|| self.ensure_and_get_gas_for_transfer_exec());
                self.perform_transfer_execute_egld_event(
                    action_id,
                    &call_data.to,
                    &call_data.egld_amount,
                    gas,
                    &call_data.endpoint_name,
                    call_data.arguments.as_multi(),
                );
                let result = self.send_raw().direct_egld_execute(
                    &call_data.to,
                    &call_data.egld_amount,
                    gas,
                    &call_data.endpoint_name,
                    &call_data.arguments.into(),
                );
                if let Result::Err(e) = result {
                    sc_panic!(e);
                }

                OptionalValue::None
            }
            Action::SendTransferExecuteEsdt(call_data) => {
                let gas = call_data
                    .opt_gas_limit
                    .unwrap_or_else(|| self.ensure_and_get_gas_for_transfer_exec());

                self.perform_transfer_execute_esdt_event(
                    action_id,
                    &call_data.to,
                    &call_data.tokens,
                    gas,
                    &call_data.endpoint_name,
                    call_data.arguments.as_multi(),
                );
                let result = self.send_raw().multi_esdt_transfer_execute(
                    &call_data.to,
                    &call_data.tokens,
                    gas,
                    &call_data.endpoint_name,
                    &call_data.arguments.into(),
                );
                if let Result::Err(e) = result {
                    sc_panic!(e);
                }

                OptionalValue::None
            }
            Action::SendAsyncCall(call_data) => {
                let gas = call_data
                    .opt_gas_limit
                    .unwrap_or_else(|| self.ensure_and_get_gas_for_transfer_exec());
                self.perform_async_call_event(
                    action_id,
                    &call_data.to,
                    &call_data.egld_amount,
                    gas,
                    &call_data.endpoint_name,
                    call_data.arguments.as_multi(),
                );
                self.send()
                    .contract_call::<()>(call_data.to, call_data.endpoint_name)
                    .with_egld_transfer(call_data.egld_amount)
                    .with_raw_arguments(call_data.arguments.into())
                    .with_gas_limit(gas)
                    .async_call()
                    .with_callback(self.callbacks().perform_async_call_callback())
                    .call_and_exit()
            }
            Action::SCDeployFromSource {
                amount,
                source,
                code_metadata,
                arguments,
            } => {
                let gas_left = self.blockchain().get_gas_left();
                self.perform_deploy_from_source_event(
                    action_id,
                    &amount,
                    &source,
                    code_metadata,
                    gas_left,
                    arguments.as_multi(),
                );
                let (new_address, _) = self.send_raw().deploy_from_source_contract(
                    gas_left,
                    &amount,
                    &source,
                    code_metadata,
                    &arguments.into(),
                );
                OptionalValue::Some(new_address)
            }
            Action::SCUpgradeFromSource {
                sc_address,
                amount,
                source,
                code_metadata,
                arguments,
            } => {
                let gas_left = self.blockchain().get_gas_left();
                self.perform_upgrade_from_source_event(
                    action_id,
                    &sc_address,
                    &amount,
                    &source,
                    code_metadata,
                    gas_left,
                    arguments.as_multi(),
                );
                self.send_raw().upgrade_from_source_contract(
                    &sc_address,
                    gas_left,
                    &amount,
                    &source,
                    code_metadata,
                    &arguments.into(),
                );
                OptionalValue::None
            }
        }
    }

    /// Callback only performs logging.
    #[callback]
    fn perform_async_call_callback(
        &self,
        #[call_result] call_result: ManagedAsyncCallResult<MultiValueEncoded<ManagedBuffer>>,
    ) {
        match call_result {
            ManagedAsyncCallResult::Ok(results) => {
                self.async_call_success(results);
            }
            ManagedAsyncCallResult::Err(err) => {
                self.async_call_error(err.err_code, err.err_msg);
            }
        }
    }

    #[event("asyncCallSuccess")]
    fn async_call_success(&self, #[indexed] results: MultiValueEncoded<ManagedBuffer>);

    #[event("asyncCallError")]
    fn async_call_error(&self, #[indexed] err_code: u32, #[indexed] err_message: ManagedBuffer);
}
