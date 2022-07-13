elrond_wasm::imports!();

/// # Elrond smart contract module - Governance
///
/// This is a standard smart contract module, that when added to a smart contract offers governance features:
/// - proposing actions
/// - voting/downvoting a certain proposal
/// - after a voting period, either putting the action in a queue (if it reached quorum), or canceling
///
/// Voting is done through energy.
///
/// The module provides the following configurable parameters:  
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
#[elrond_wasm::module]
pub trait ConfigurablePropertiesModule {
    #[init]
    fn init_governance_module(
        &self,
        quorum: BigUint,
        voting_delay_in_blocks: u64,
        voting_period_in_blocks: u64,
        lock_time_after_voting_ends_in_blocks: u64,
    ) {
        self.try_change_quorum(quorum);
        self.try_change_voting_delay_in_blocks(voting_delay_in_blocks);
        self.try_change_voting_period_in_blocks(voting_period_in_blocks);
        self.try_change_lock_time_after_voting_ends_in_blocks(
            lock_time_after_voting_ends_in_blocks,
        );
    }

    // endpoints - these can only be called by the SC itself.
    // i.e. only by proposing and executing an action with the SC as dest and the respective func name

    #[endpoint(changeQuorum)]
    fn change_quorum(&self, new_value: BigUint) {
        self.require_caller_self();

        self.try_change_quorum(new_value);
    }

    #[endpoint(changeVotingDelayInBlocks)]
    fn change_voting_delay_in_blocks(&self, new_value: u64) {
        self.require_caller_self();

        self.try_change_voting_delay_in_blocks(new_value);
    }

    #[endpoint(changeVotingPeriodInBlocks)]
    fn change_voting_period_in_blocks(&self, new_value: u64) {
        self.require_caller_self();

        self.try_change_voting_period_in_blocks(new_value);
    }

    #[endpoint(changeLockTimeAfterVotingEndsInBlocks)]
    fn change_lock_time_after_voting_ends_in_blocks(&self, new_value: u64) {
        self.require_caller_self();

        self.try_change_lock_time_after_voting_ends_in_blocks(new_value);
    }

    fn require_caller_self(&self) {
        let caller = self.blockchain().get_caller();
        let sc_address = self.blockchain().get_sc_address();

        require!(
            caller == sc_address,
            "Only the SC itself may call this function"
        );
    }

    fn try_change_quorum(&self, new_value: BigUint) {
        require!(new_value != 0, "Quorum can't be set to 0");

        self.quorum().set(&new_value);
    }

    fn try_change_voting_delay_in_blocks(&self, new_value: u64) {
        require!(new_value != 0, "Voting delay in blocks can't be set to 0");

        self.voting_delay_in_blocks().set(&new_value);
    }

    fn try_change_voting_period_in_blocks(&self, new_value: u64) {
        require!(
            new_value != 0,
            "Voting period (in blocks) can't be set to 0"
        );

        self.voting_period_in_blocks().set(&new_value);
    }

    fn try_change_lock_time_after_voting_ends_in_blocks(&self, new_value: u64) {
        require!(
            new_value != 0,
            "Lock time after voting ends (in blocks) can't be set to 0"
        );

        self.lock_time_after_voting_ends_in_blocks().set(&new_value);
    }

    #[view(getQuorum)]
    #[storage_mapper("quorum")]
    fn quorum(&self) -> SingleValueMapper<BigUint>;

    #[view(getVotingDelayInBlocks)]
    #[storage_mapper("votingDelayInBlocks")]
    fn voting_delay_in_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getVotingPeriodInBlocks)]
    #[storage_mapper("votingPeriodInBlocks")]
    fn voting_period_in_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getLockTimeAfterVotingEndsInBlocks)]
    #[storage_mapper("lockTimeAfterVotingEndsInBlocks")]
    fn lock_time_after_voting_ends_in_blocks(&self) -> SingleValueMapper<u64>;
}
