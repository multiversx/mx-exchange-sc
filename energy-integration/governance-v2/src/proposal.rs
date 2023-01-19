multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const MAX_GOVERNANCE_PROPOSAL_ACTIONS: usize = 4;

pub type ProposalId = usize;

pub type GovernanceActionAsMultiArg<M> =
    MultiValue4<u64, ManagedAddress<M>, ManagedBuffer<M>, ManagedVec<M, ManagedBuffer<M>>>;

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Eq)]
pub enum GovernanceProposalStatus {
    None,
    Pending,
    Active,
    Defeated,
    Succeeded,
    Queued,
    WaitingForFees,
}
#[derive(
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    ManagedVecItem,
    TypeAbi,
    PartialEq,
    Debug,
    Clone,
)]
pub struct ProposalFees<M: ManagedTypeApi> {
    pub total_amount: BigUint<M>,
    pub entries: ManagedVec<M, FeeEntry<M>>,
}

#[derive(
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    ManagedVecItem,
    TypeAbi,
    PartialEq,
    Debug,
    Clone,
)]
pub struct FeeEntry<M: ManagedTypeApi> {
    pub depositor_addr: ManagedAddress<M>,
    pub tokens: EsdtTokenPayment<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
pub struct GovernanceAction<M: ManagedTypeApi> {
    pub gas_limit: u64,
    pub dest_address: ManagedAddress<M>,
    pub function_name: ManagedBuffer<M>,
    pub arguments: ManagedVec<M, ManagedBuffer<M>>,
}

impl<M: ManagedTypeApi> GovernanceAction<M> {
    pub fn into_multiarg(self) -> GovernanceActionAsMultiArg<M> {
        (
            self.gas_limit,
            self.dest_address,
            self.function_name,
            self.arguments,
        )
            .into()
    }
}

impl<M: ManagedTypeApi> From<GovernanceActionAsMultiArg<M>> for GovernanceAction<M> {
    fn from(multi_arg: GovernanceActionAsMultiArg<M>) -> Self {
        let (gas_limit, dest_address, function_name, arguments) = multi_arg.into_tuple();
        GovernanceAction {
            gas_limit,
            dest_address,
            function_name,
            arguments,
        }
    }
}

#[derive(TypeAbi, TopEncode, TopDecode, PartialEq, Debug)]
pub struct GovernanceProposal<M: ManagedTypeApi> {
    pub proposer: ManagedAddress<M>,
    pub actions: ArrayVec<GovernanceAction<M>, MAX_GOVERNANCE_PROPOSAL_ACTIONS>,
    pub description: ManagedBuffer<M>,
    pub fees: ProposalFees<M>,
}
