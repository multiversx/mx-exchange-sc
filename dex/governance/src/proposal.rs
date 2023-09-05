multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use crate::config;
use crate::errors::*;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub enum ProposalStatus {
    Pending, //Starts from 0
    Active,
    Defeated,
    Succeeded,
    Executed,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, TypeAbi)]
pub struct Action<M: ManagedTypeApi> {
    pub gas_limit: u64,
    pub dest_address: ManagedAddress<M>,
    pub payments: ManagedVec<M, ManagedBuffer<M>>,
    pub endpoint_name: ManagedBuffer<M>,
    pub arguments: ManagedVec<M, ManagedBuffer<M>>,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct ProposalCreationArgs<M: ManagedTypeApi> {
    pub description: ManagedBuffer<M>,
    pub actions: ManagedVec<M, Action<M>>,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct Proposal<M: ManagedTypeApi> {
    pub id: u64,
    pub creation_block: u64,
    pub proposer: ManagedAddress<M>,
    pub description: ManagedBuffer<M>,

    pub was_executed: bool,
    pub actions: ManagedVec<M, Action<M>>,

    pub num_upvotes: BigUint<M>,
    pub num_downvotes: BigUint<M>,
}

#[multiversx_sc::module]
pub trait ProposalHelper: config::Config {
    #[view(getProposalStatus)]
    fn get_proposal_status_view(&self, proposal_id: u64) -> ProposalStatus {
        require!(!self.proposal(proposal_id).is_empty(), PROPOSAL_NOT_FOUND);
        let proposal = self.proposal(proposal_id).get();

        self.get_proposal_status(&proposal)
    }

    fn get_proposal_status(&self, proposal: &Proposal<Self::Api>) -> ProposalStatus {
        if proposal.was_executed {
            return ProposalStatus::Executed;
        }

        let current_block = self.blockchain().get_block_nonce();
        let proposal_block = proposal.creation_block;
        let voting_delay = self.voting_delay_in_blocks().get();
        let voting_period = self.voting_period_in_blocks().get();

        let voting_start = proposal_block + voting_delay;
        let voting_end = voting_start + voting_period;

        if current_block < voting_start {
            return ProposalStatus::Pending;
        }
        if current_block >= voting_start && current_block < voting_end {
            return ProposalStatus::Active;
        }

        let total_upvotes = &proposal.num_upvotes;
        let total_downvotes = &proposal.num_downvotes;
        let quorum = self.quorum().get();

        if total_upvotes > total_downvotes && total_upvotes - total_downvotes >= quorum {
            ProposalStatus::Succeeded
        } else {
            ProposalStatus::Defeated
        }
    }

    fn new_proposal_from_args(&self, args: ProposalCreationArgs<Self::Api>) -> Proposal<Self::Api> {
        Proposal {
            id: self.proposal_id_counter().get(),
            creation_block: self.blockchain().get_block_nonce(),
            proposer: self.blockchain().get_caller(),
            description: args.description,
            was_executed: false,
            actions: args.actions,
            num_upvotes: BigUint::zero(),
            num_downvotes: BigUint::zero(),
        }
    }

    fn execute_proposal(&self, proposal: &Proposal<Self::Api>) {
        for action in proposal.actions.iter() {
            self.execute_action(&action).unwrap()
        }
    }

    fn execute_action(&self, action: &Action<Self::Api>) -> Result<(), &'static [u8]> {
        self.send()
            .contract_call::<()>(action.dest_address.clone(), action.endpoint_name.clone())
            .with_raw_arguments(ManagedArgBuffer::from(action.arguments.clone()))
            .with_gas_limit(action.gas_limit)
            .transfer_execute();
        Result::Ok(())
        // ContractCallNoPayment::new()
        // self.send_raw().direct_egld(
        //     &BigUint::zero(),
        //     action.gas_limit,
        //     &action.endpoint_name,
        //     &ManagedArgBuffer::from(action.arguments.clone()),
        // )
    }
}
