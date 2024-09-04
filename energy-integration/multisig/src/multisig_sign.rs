use crate::multisig_state::{ActionId, ActionStatus, GroupId};

use multiversx_sc::imports::*;

#[multiversx_sc::module]
pub trait MultisigSignModule:
    crate::multisig_state::MultisigStateModule
    + crate::multisig_propose::MultisigProposeModule
    + crate::multisig_perform::MultisigPerformModule
    + crate::multisig_events::MultisigEventsModule
{
    /// Used by board members to sign actions.
    #[endpoint]
    fn sign(&self, action_id: ActionId) {
        require!(
            !self.action_mapper().item_is_empty_unchecked(action_id),
            "action does not exist"
        );
        let group_id = self.group_for_action(action_id).get();
        if group_id != 0 {
            let group_status = self.action_group_status(group_id).get();
            require!(
                group_status == ActionStatus::Available,
                "cannot sign actions of an aborted batch"
            );
        }
        let (caller_id, caller_role) = self.get_caller_id_and_role();
        require!(caller_role.can_sign(), "only board members can sign");

        let _ = self.action_signer_ids(action_id).insert(caller_id);
    }

    /// Sign all the actions in the given batch
    #[endpoint(signBatch)]
    fn sign_batch(&self, group_id: GroupId) {
        let (caller_id, caller_role) = self.get_caller_id_and_role();
        require!(caller_role.can_sign(), "only board members can sign");

        let group_status = self.action_group_status(group_id).get();
        require!(
            group_status == ActionStatus::Available,
            "cannot sign actions of an aborted batch"
        );
        let mapper = self.action_groups(group_id);
        require!(!mapper.is_empty(), "Invalid group ID");

        for action_id in mapper.iter() {
            require!(
                !self.action_mapper().item_is_empty_unchecked(action_id),
                "action does not exist"
            );

            let _ = self.action_signer_ids(action_id).insert(caller_id);
        }
    }

    #[endpoint(signAndPerform)]
    fn sign_and_perform(&self, action_id: ActionId) -> OptionalValue<ManagedAddress> {
        self.sign(action_id);
        self.try_perform_action(action_id)
    }

    #[endpoint(signBatchAndPerform)]
    fn sign_batch_and_perform(&self, group_id: GroupId) {
        self.sign_batch(group_id);

        let (_, caller_role) = self.get_caller_id_and_role();
        require!(
            caller_role.can_perform_action(),
            "only board members and proposers can perform actions"
        );

        let mut quorums_reached = true;

        for action_id in self.action_groups(group_id).iter() {
            if !self.quorum_reached(action_id) {
                quorums_reached = false;
            }
        }

        if !quorums_reached {
            return;
        }

        // Copy action_ids before executing them since perform_action does a swap_remove
        //   clearing the last item
        let mut action_ids = ManagedVec::<Self::Api, _>::new();
        for action_id in self.action_groups(group_id).iter() {
            action_ids.push(action_id);
        }

        for action_id in &action_ids {
            let _ = self.perform_action(action_id);
        }
    }

    /// Board members can withdraw their signatures if they no longer desire for the action to be executed.
    /// Actions that are left with no valid signatures can be then deleted to free up storage.
    #[endpoint]
    fn unsign(&self, action_id: ActionId) {
        let (caller_id, caller_role) = self.get_caller_id_and_role();
        require!(caller_role.can_sign(), "only board members can un-sign");
        self.unsign_action(action_id, caller_id);
    }

    /// Unsign all actions with the given IDs
    #[endpoint(unsignBatch)]
    fn unsign_batch(&self, group_id: GroupId) {
        let (caller_id, caller_role) = self.get_caller_id_and_role();
        require!(caller_role.can_sign(), "only board members can un-sign");

        let mapper = self.action_groups(group_id);
        require!(!mapper.is_empty(), "Invalid group ID");

        for action_id in mapper.iter() {
            self.unsign_action(action_id, caller_id);
        }
    }

    fn unsign_action(&self, action_id: ActionId, caller_id: usize) {
        require!(
            !self.action_mapper().item_is_empty_unchecked(action_id),
            "action does not exist"
        );

        let _ = self.action_signer_ids(action_id).swap_remove(&caller_id);
    }

    /// Returns `true` (`1`) if the user has signed the action.
    /// Does not check whether or not the user is still a board member and the signature valid.
    #[view]
    fn signed(&self, user: ManagedAddress, action_id: ActionId) -> bool {
        let user_id = self.user_mapper().get_user_id(&user);
        if user_id == 0 {
            false
        } else {
            self.action_signer_ids(action_id).contains(&user_id)
        }
    }

    #[endpoint(unsignForOutdatedBoardMembers)]
    fn unsign_for_outdated_board_members(
        &self,
        action_id: ActionId,
        outdated_board_members: MultiValueEncoded<usize>,
    ) {
        let mut board_members_to_remove: ManagedVec<usize> = ManagedVec::new();
        if outdated_board_members.is_empty() {
            for signer_id in self.action_signer_ids(action_id).iter() {
                if !self.user_id_to_role(signer_id).get().can_sign() {
                    board_members_to_remove.push(signer_id);
                }
            }
        } else {
            for signer_id in outdated_board_members.into_iter() {
                if !self.user_id_to_role(signer_id).get().can_sign() {
                    board_members_to_remove.push(signer_id);
                }
            }
        }
        for member in board_members_to_remove.iter() {
            self.action_signer_ids(action_id).swap_remove(&member);
        }
    }
}
