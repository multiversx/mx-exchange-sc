elrond_wasm::imports!();
elrond_wasm::derive_imports!();

pub const HASH_LENGTH: usize = 32;
pub const PROOF_LENGTH: usize = 18;
pub const MAX_GOVERNANCE_PROPOSAL_ACTIONS: usize = 5;

pub type ProposalId = usize;

pub type GovernanceActionAsMultiArg<M> = MultiValue5<
    u64,
    ManagedAddress<M>,
    ManagedVec<M, EsdtTokenPayment<M>>,
    ManagedBuffer<M>,
    ManagedVec<M, ManagedBuffer<M>>,
>;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Eq)]
pub enum GovernanceProposalStatus {
    None,
    Pending,
    Active,
    Defeated,
    Succeeded,
    Queued,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode)]
pub struct GovernanceAction<M: ManagedTypeApi> {
    pub gas_limit: u64,
    pub dest_address: ManagedAddress<M>,
    pub payments: ManagedVec<M, EsdtTokenPayment<M>>,
    pub function_name: ManagedBuffer<M>,
    pub arguments: ManagedVec<M, ManagedBuffer<M>>,
}

impl<M: ManagedTypeApi> GovernanceAction<M> {
    pub fn into_multiarg(self) -> GovernanceActionAsMultiArg<M> {
        (
            self.gas_limit,
            self.dest_address,
            self.payments,
            self.function_name,
            self.arguments,
        )
            .into()
    }
}

impl<M: ManagedTypeApi> From<GovernanceActionAsMultiArg<M>> for GovernanceAction<M> {
    fn from(multi_arg: GovernanceActionAsMultiArg<M>) -> Self {
        let (gas_limit, dest_address, payments, function_name, arguments) = multi_arg.into_tuple();
        GovernanceAction {
            gas_limit,
            dest_address,
            payments,
            function_name,
            arguments,
        }
    }
}

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct GovernanceProposal<M: ManagedTypeApi> {
    pub proposer: ManagedAddress<M>,
    pub actions: ArrayVec<GovernanceAction<M>, MAX_GOVERNANCE_PROPOSAL_ACTIONS>,
    pub description: ManagedBuffer<M>,
    pub root_hash: ManagedByteArray<M, HASH_LENGTH>,
}
