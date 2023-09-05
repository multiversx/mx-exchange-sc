use crate::{errors::ERROR_NOT_AN_ESDT, FULL_PERCENTAGE};

multiversx_sc::imports!();

/// # MultiversX smart contract module - Governance
///
/// This is a standard smart contract module, that when added to a smart contract offers governance features:
/// - proposing actions
/// - voting/downvoting a certain proposal
/// - after a voting period, either putting the action in a queue (if it reached quorum), or canceling
///
/// Voting is done through energy.
///
/// The module provides the following configurable parameters:  
/// - `minEnergyForPropose` - the minimum energy required for submitting a proposal
/// - `quorum` - the minimum number of (`votes` minus `downvotes`) at the end of voting period  
/// - `maxActionsPerProposal` - Maximum number of actions (transfers and/or smart contract calls) that a proposal may have  
/// - `votingDelayInBlocks` - Number of blocks to wait after a block is proposed before being able to vote/downvote that proposal
/// - `votingPeriodInBlocks` - Number of blocks the voting period lasts (voting delay does not count towards this)  
/// - `lockTimeAfterVotingEndsInBlocks` - Number of blocks to wait before a successful proposal can be executed  
///
/// The module also provides events for most actions that happen:
/// - `proposalCreated` - triggers when a proposal is created. Also provoides all the relevant information, like proposer, actions etc.  
/// - `voteCast` - user voted on a proposal  
/// - `downvoteCast` - user downvoted a proposal  
/// - `proposalCanceled`, `proposalQueued` and `proposalExecuted` - provides the ID of the specific proposal  
/// - `userDeposit` - a user deposited some tokens needed for a future payable action  
///
/// Please note that although the main contract can modify the module's storage directly, it is not recommended to do so,
/// as that defeats the whole purpose of having governance. These parameters should only be modified through actions.
///

const MIN_VOTING_DELAY: u64 = 1;
const MAX_VOTING_DELAY: u64 = 100_800; // 1 Week
const MIN_VOTING_PERIOD: u64 = 14_400; // 24 Hours
const MAX_VOTING_PERIOD: u64 = 201_600; // 2 Weeks
const MIN_QUORUM: u64 = 1_000; // 10%
const MAX_QUORUM: u64 = 6_000; // 60%
const MIN_MIN_FEE_FOR_PROPOSE: u64 = 2_000_000;
const MAX_MIN_FEE_FOR_PROPOSE: u64 = 200_000_000_000;
const DECIMALS_CONST: u64 = 1_000_000_000_000_000_000;

#[multiversx_sc::module]
pub trait ConfigurablePropertiesModule:
    energy_query::EnergyQueryModule + permissions_module::PermissionsModule
{
    // endpoints - these can only be called by the SC itself.
    // i.e. only by proposing and executing an action with the SC as dest and the respective func name

    #[only_owner]
    #[endpoint(changeMinEnergyForProposal)]
    fn change_min_energy_for_propose(&self, new_value: BigUint) {
        self.try_change_min_energy_for_propose(new_value);
    }

    #[only_owner]
    #[endpoint(changeMinFeeForProposal)]
    fn change_min_fee_for_propose(&self, new_value: BigUint) {
        self.try_change_min_fee_for_propose(new_value);
    }

    #[only_owner]
    #[endpoint(changeQuorumPercentage)]
    fn change_quorum_percentage(&self, new_value: BigUint) {
        self.try_change_quorum_percentage(new_value);
    }

    #[only_owner]
    #[endpoint(changeVotingDelayInBlocks)]
    fn change_voting_delay_in_blocks(&self, new_value: u64) {
        self.try_change_voting_delay_in_blocks(new_value);
    }

    #[only_owner]
    #[endpoint(changeVotingPeriodInBlocks)]
    fn change_voting_period_in_blocks(&self, new_value: u64) {
        self.try_change_voting_period_in_blocks(new_value);
    }

    fn try_change_min_energy_for_propose(&self, new_value: BigUint) {
        require!(new_value != 0, "Min energy for proposal can't be set to 0");

        self.min_energy_for_propose().set(&new_value);
    }

    fn try_change_min_fee_for_propose(&self, new_value: BigUint) {
        let minimum_min_fee =
            BigUint::from(MIN_MIN_FEE_FOR_PROPOSE) * BigUint::from(DECIMALS_CONST);
        let maximum_min_fee =
            BigUint::from(MAX_MIN_FEE_FOR_PROPOSE) * BigUint::from(DECIMALS_CONST);
        require!(
            new_value > minimum_min_fee && new_value < maximum_min_fee,
            "Not valid value for min fee!"
        );

        self.min_fee_for_propose().set(&new_value);
    }

    fn try_change_quorum_percentage(&self, new_value: BigUint) {
        require!(
            new_value > MIN_QUORUM && new_value < MAX_QUORUM,
            "Not valid value for Quorum!"
        );

        self.quorum_percentage().set(&new_value);
    }

    fn try_change_voting_delay_in_blocks(&self, new_value: u64) {
        require!(
            new_value > MIN_VOTING_DELAY && new_value < MAX_VOTING_DELAY,
            "Not valid value for voting delay!"
        );

        self.voting_delay_in_blocks().set(new_value);
    }

    fn try_change_voting_period_in_blocks(&self, new_value: u64) {
        require!(
            new_value > MIN_VOTING_PERIOD && new_value < MAX_VOTING_PERIOD,
            "Not valid value for voting period!"
        );

        self.voting_period_in_blocks().set(new_value);
    }

    fn try_change_withdraw_percentage_defeated(&self, new_value: u64) {
        require!(
            new_value > 0 && new_value < FULL_PERCENTAGE,
            "Not valid value for withdraw percentage if defeated!"
        );

        self.withdraw_percentage_defeated().set(new_value);
    }

    fn try_change_fee_token_id(&self, fee_token_id: TokenIdentifier) {
        require!(fee_token_id.is_valid_esdt_identifier(), ERROR_NOT_AN_ESDT);
        self.fee_token_id().set_if_empty(&fee_token_id);
    }

    fn smoothing_function(&self, input: &BigUint) -> BigUint {
        input.sqrt()
    }

    #[view(getMinEnergyForPropose)]
    #[storage_mapper("minEnergyForPropose")]
    fn min_energy_for_propose(&self) -> SingleValueMapper<BigUint>;

    #[view(getMinFeeForPropose)]
    #[storage_mapper("minFeeForPropose")]
    fn min_fee_for_propose(&self) -> SingleValueMapper<BigUint>;

    #[view(getQuorum)]
    #[storage_mapper("quorumPercentage")]
    fn quorum_percentage(&self) -> SingleValueMapper<BigUint>;

    #[view(getVotingDelayInBlocks)]
    #[storage_mapper("votingDelayInBlocks")]
    fn voting_delay_in_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getVotingPeriodInBlocks)]
    #[storage_mapper("votingPeriodInBlocks")]
    fn voting_period_in_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getFeeTokenId)]
    #[storage_mapper("feeTokenId")]
    fn fee_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getWithdrawPercentageDefeated)]
    #[storage_mapper("witdrawPercentageDefeated")]
    fn withdraw_percentage_defeated(&self) -> SingleValueMapper<u64>;
}
