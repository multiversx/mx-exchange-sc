multiversx_sc::imports!();

use crate::{
    errors::*,
    proposal::GovernanceProposalStatus,
    proposal::{FeeEntry, ProposalId},
};

const MIN_AMOUNT_PER_DEPOSIT: u64 = 1;

#[multiversx_sc::module]
pub trait GovFeesModule:
    crate::configurable::ConfigurablePropertiesModule
    + crate::proposal_storage::ProposalStorageModule
    + crate::events::EventsModule
    + crate::views::ViewsModule
    + crate::caller_check::CallerCheckModule
    + energy_query::EnergyQueryModule
{
    /// Used to deposit tokens to gather threshold min_fee.
    /// Funds will be returned if the proposal is canceled.
    #[payable("*")]
    #[endpoint(depositTokensForProposal)]
    fn deposit_tokens_for_proposal(&self, proposal_id: ProposalId) {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::WaitingForFees,
            "Proposal is not waiting for fees anymore"
        );

        require!(
            !self.proposal_reached_min_fees(proposal_id),
            MIN_FEES_REACHED
        );

        let additional_fee = self.call_value().single_esdt();
        require!(
            self.fee_token_id().get() == additional_fee.token_identifier,
            WRONG_TOKEN_ID
        );
        require!(
            additional_fee.amount >= MIN_AMOUNT_PER_DEPOSIT,
            MIN_AMOUNT_NOT_REACHED
        );

        let caller = self.blockchain().get_caller();
        let mut proposal = self.proposals().get(proposal_id);
        proposal.fees.entries.push(FeeEntry {
            depositor_addr: caller.clone(),
            tokens: additional_fee.clone(),
        });
        proposal.fees.total_amount += additional_fee.amount.clone();

        self.proposals().set(proposal_id, &proposal);
        self.user_deposit_event(&caller, proposal_id, &additional_fee);
    }

    /// Used to claim deposited tokens to gather threshold min_fee.
    #[payable("*")]
    #[endpoint(claimDepositedTokens)]
    fn claim_deposited_tokens(&self, proposal_id: ProposalId) -> ManagedVec<EsdtTokenPayment> {
        self.require_caller_not_self();
        self.require_valid_proposal_id(proposal_id);
        require!(
            self.get_proposal_status(proposal_id) == GovernanceProposalStatus::WaitingForFees,
            "Cannot claim deposited tokens anymore; Proposal is not in WatingForFees state"
        );
        require!(
            !self.proposal_reached_min_fees(proposal_id),
            MIN_FEES_REACHED
        );

        let caller = self.blockchain().get_caller();
        let mut proposal = self.proposals().get(proposal_id);
        let entries = &mut proposal.fees.entries;

        let mut fees_to_send = ManagedVec::new();
        let mut total_fees = BigUint::zero();
        let mut i = 0;
        let mut entries_len = entries.len();
        while i < entries_len {
            let entry = entries.get(i);
            if entry.depositor_addr == caller {
                total_fees += &entry.tokens.amount;
                entries_len -= 1;

                fees_to_send.push(entry.tokens);
                entries.remove(i);
            } else {
                i += 1;
            }
        }

        require!(total_fees > 0, "No tokens to send");

        proposal.fees.total_amount -= total_fees;
        self.proposals().set(proposal_id, &proposal);

        self.send().direct_multi(&caller, &fees_to_send);
        self.user_claim_deposited_tokens_event(&caller, proposal_id, &fees_to_send);

        fees_to_send
    }
}
