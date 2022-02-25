elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq)]
pub enum ProposalStatus {
    Pending = 1,
    Active = 2,
    Defeated = 3,
    Succeeded = 4,
    Expired = 5,
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
pub struct Proposal<M: ManagedTypeApi> {
    pub creation_block: u64,
    pub proposer: ManagedAddress<M>,
    pub description: ManagedBuffer<M>,
    pub actions: ManagedVec<M, Action<M>>,

    pub num_votes: BigUint<M>,
    pub num_downvotes: BigUint<M>,
    pub funds: ManagedVec<M, EsdtTokenPayment<M>>,
}

#[elrond_wasm::module]
pub trait ProposalHelper {
    #[view(getProposalStatus)]
    fn get_proposal_status(&self, _proposal: Proposal<Self::Api>) -> ProposalStatus {
        unreachable!();
    }
}
