elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use crate::config;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq)]
pub enum ProposalStatus {
    Pending = 1,
    Active = 2,
    Defeated = 3,
    Succeeded = 4,
    Expired = 5,
    Executed = 6,
}

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, ManagedVecItem, TypeAbi)]
pub struct Action<M: ManagedTypeApi> {
    pub gas_limit: u64,
    pub dest_address: ManagedAddress<M>,
    pub payments: ManagedVec<M, ManagedBuffer<M>>,
    pub function_name: ManagedBuffer<M>,
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

    pub executed: bool,
    pub actions: ManagedVec<M, Action<M>>,

    pub num_upvotes: BigUint<M>,
    pub num_downvotes: BigUint<M>,
}

#[elrond_wasm::module]
pub trait ProposalHelper: config::Config {
    #[view(getProposalStatus)]
    fn get_proposal_status(&self, _proposal: &Proposal<Self::Api>) -> ProposalStatus {
        todo!();
    }

    fn new_proposal_from_args(&self, args: ProposalCreationArgs<Self::Api>) -> Proposal<Self::Api> {
        Proposal {
            id: self.proposal_id_counter().get(),
            creation_block: self.blockchain().get_block_nonce(),
            proposer: self.blockchain().get_caller(),
            description: args.description,
            executed: false,
            actions: args.actions,
            num_upvotes: BigUint::zero(),
            num_downvotes: BigUint::zero(),
        }
    }

    fn execute_proposal(&self, _proposal: &Proposal<Self::Api>) {
        todo!()
    }
}
