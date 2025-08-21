use multiversx_sc::codec::{DecodeDefault, EncodeDefault};

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

pub const MAX_GOVERNANCE_PROPOSAL_ACTIONS: usize = 4;

pub type ProposalId = usize;

pub type GovernanceActionAsMultiArg<M> =
    MultiValue4<u64, ManagedAddress<M>, ManagedBuffer<M>, ManagedVec<M, ManagedBuffer<M>>>;

#[type_abi]
#[derive(TopEncode, TopDecode, PartialEq, Eq)]
pub enum GovernanceProposalStatus {
    None,
    Pending,
    Active,
    Defeated,
    DefeatedWithVeto,
    Succeeded,
}

#[type_abi]
#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, Debug)]
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

#[type_abi]
#[derive(NestedEncode, NestedDecode, PartialEq, Debug, TopEncodeOrDefault, TopDecodeOrDefault)]
pub struct GovernanceProposal<M: ManagedTypeApi> {
    pub proposal_id: usize,
    pub proposer: ManagedAddress<M>,
    pub actions: ArrayVec<GovernanceAction<M>, MAX_GOVERNANCE_PROPOSAL_ACTIONS>,
    pub description: ManagedBuffer<M>,
    pub fee_payment: EsdtTokenPayment<M>,
    pub minimum_quorum: u64,
    pub voting_delay_in_blocks: u64,
    pub voting_period_in_blocks: u64,
    pub withdraw_percentage_defeated: u64,
    pub total_quorum: BigUint<M>,
    pub proposal_start_block: u64,
    pub fee_withdrawn: bool,
}

impl<M: ManagedTypeApi> EncodeDefault for GovernanceProposal<M> {
    fn is_default(&self) -> bool {
        self.proposal_id == 0
    }
}

impl<M: ManagedTypeApi> DecodeDefault for GovernanceProposal<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: ManagedTypeApi> Default for GovernanceProposal<M> {
    fn default() -> Self {
        Self::new()
    }
}

impl<M: ManagedTypeApi> GovernanceProposal<M> {
    pub fn new() -> Self {
        GovernanceProposal {
            proposal_id: 0,
            proposer: ManagedAddress::default(),
            actions: ArrayVec::default(),
            description: ManagedBuffer::default(),
            fee_payment: EsdtTokenPayment {
                token_identifier: TokenIdentifier::from(""),
                token_nonce: 0,
                amount: BigUint::zero(),
            },
            minimum_quorum: 0,
            voting_delay_in_blocks: 0,
            voting_period_in_blocks: 0,
            withdraw_percentage_defeated: 0,
            total_quorum: BigUint::default(),
            proposal_start_block: 0,
            fee_withdrawn: false,
        }
    }
}
