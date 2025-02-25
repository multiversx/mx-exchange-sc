#![allow(deprecated)]

use energy_factory_mock::EnergyFactoryMock;
use energy_query::Energy;
use fees_collector::{claim::ClaimModule, FeesCollector};
use governance_v2::{
    configurable::ConfigurablePropertiesModule,
    proposal_storage::{ProposalStorageModule, VoteType},
    GovernanceV2,
};
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    types::{Address, BigInt, EsdtLocalRole, ManagedVec, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    whitebox_legacy::TxResult,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use num_bigint::BigUint;

pub const MIN_ENERGY_FOR_PROPOSE: u64 = 0;
pub const MIN_FEE_FOR_PROPOSE: u64 = 1_000_000_000; // 1B MEX
pub const QUORUM_PERCENTAGE: u64 = 4_000; // 40%
pub const VOTING_DELAY_BLOCKS: u64 = 1;
pub const VOTING_PERIOD_BLOCKS: u64 = 144_000; // 10 days
pub const LOCKING_PERIOD_BLOCKS: u64 = 30;
pub const WITHDRAW_PERCENTAGE: u64 = 5_000; // 50%
pub const MEX_TOKEN_ID: &[u8] = b"MEX-123456";
pub const XMEX_TOKEN_ID: &[u8] = b"XMEX-123456";
pub const DECIMALS_CONST: u64 = 1_000_000_000_000_000_000;
pub const FULL_PERCENTAGE: u64 = 10_000;
pub const USER_ENERGY: u64 = 1_000_000;
pub const GAS_LIMIT: u64 = 1_000_000;

pub struct GovSetup<GovBuilder>
where
    GovBuilder: 'static + Copy + Fn() -> governance_v2::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub third_user: Address,
    pub no_energy_user: Address,
    pub gov_wrapper: ContractObjWrapper<governance_v2::ContractObj<DebugApi>, GovBuilder>,
    pub current_block: u64,
}

impl<GovBuilder> GovSetup<GovBuilder>
where
    GovBuilder: 'static + Copy + Fn() -> governance_v2::ContractObj<DebugApi>,
{
    pub fn new(gov_builder: GovBuilder) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let third_user = b_mock.create_user_account(&rust_zero);
        let no_energy_user = b_mock.create_user_account(&rust_zero);

        // init energy factory
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_mock::contract_obj,
            "energy factory path",
        );

        // init fees collector
        let fees_collector_wrapper = b_mock.create_sc_account(
            &rust_biguint!(0),
            None,
            fees_collector::contract_obj,
            "fees collector path",
        );

        b_mock
            .execute_tx(&owner, &energy_factory_wrapper, &rust_zero, |sc| {
                sc.init();
                sc.user_energy(&managed_address!(&first_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(USER_ENERGY)),
                        0,
                        managed_biguint!(0),
                    ));
                sc.user_energy(&managed_address!(&second_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(USER_ENERGY)),
                        0,
                        managed_biguint!(0),
                    ));
                sc.user_energy(&managed_address!(&third_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(USER_ENERGY + 210_000)),
                        0,
                        managed_biguint!(0),
                    ));
                sc.user_energy(&managed_address!(&no_energy_user))
                    .set(&Energy::new(
                        BigInt::from(managed_biguint!(0)),
                        0,
                        managed_biguint!(0),
                    ));
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &fees_collector_wrapper, &rust_biguint!(0), |sc| {
                sc.init(
                    managed_token_id!(XMEX_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                    managed_address!(energy_factory_wrapper.address_ref()), // unused
                    MultiValueEncoded::new(),
                );
            })
            .assert_ok();

        b_mock
            .execute_tx(
                &first_user,
                &fees_collector_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.claim_rewards_endpoint(OptionalValue::None);
                },
            )
            .assert_ok();

        b_mock
            .execute_tx(
                &second_user,
                &fees_collector_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.claim_rewards_endpoint(OptionalValue::None);
                },
            )
            .assert_ok();

        b_mock
            .execute_tx(
                &third_user,
                &fees_collector_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.claim_rewards_endpoint(OptionalValue::None);
                },
            )
            .assert_ok();

        // init governance sc
        let gov_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), gov_builder, "gov path");

        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_biguint!(MIN_ENERGY_FOR_PROPOSE),
                    managed_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST,
                    QUORUM_PERCENTAGE,
                    VOTING_DELAY_BLOCKS,
                    VOTING_PERIOD_BLOCKS,
                    WITHDRAW_PERCENTAGE,
                    managed_address!(energy_factory_wrapper.address_ref()),
                    managed_address!(fees_collector_wrapper.address_ref()),
                    managed_token_id!(MEX_TOKEN_ID),
                );
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &gov_wrapper, &rust_zero, |sc| {
                sc.fee_token_id().set(managed_token_id!(MEX_TOKEN_ID));
            })
            .assert_ok();

        let vote_nft_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
        b_mock.set_esdt_local_roles(gov_wrapper.address_ref(), MEX_TOKEN_ID, &vote_nft_roles[..]);

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            third_user,
            no_energy_user,
            gov_wrapper,
            current_block: 0,
        }
    }

    pub fn propose(
        &mut self,
        proposer: &Address,
        fee_amount: &BigUint,
        dest_address: &Address,
        endpoint_name: &[u8],
        args: Vec<Vec<u8>>,
    ) -> (TxResult, usize) {
        let mut proposal_id = 0;
        let result = self.b_mock.execute_esdt_transfer(
            proposer,
            &self.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            fee_amount,
            |sc| {
                let mut args_managed = ManagedVec::new();
                for arg in args {
                    args_managed.push(managed_buffer!(&arg));
                }

                let mut actions = MultiValueEncoded::new();
                actions.push(
                    (
                        GAS_LIMIT,
                        managed_address!(dest_address),
                        managed_buffer!(endpoint_name),
                        args_managed,
                    )
                        .into(),
                );

                proposal_id = sc.propose(managed_buffer!(b"changeTODO"), actions);
            },
        );

        (result, proposal_id)
    }

    pub fn up_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::UpVote);
            })
    }

    pub fn down_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::DownVote);
            })
    }

    pub fn down_veto_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::DownVetoVote);
            })
    }

    pub fn abstain_vote(&mut self, voter: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(voter, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.vote(proposal_id, VoteType::AbstainVote);
            })
    }

    pub fn withdraw_after_defeated(&mut self, caller: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.withdraw_deposit(proposal_id);
            })
    }

    pub fn change_withdraw_percentage(&mut self, withdraw_value: u64) -> TxResult {
        self.b_mock
            .execute_tx(&self.owner, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.change_withdraw_percentage(withdraw_value);
            })
    }

    pub fn change_min_energy(&mut self, min_energy_for_propose: usize) -> TxResult {
        self.b_mock
            .execute_tx(&self.owner, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.min_energy_for_propose()
                    .set(&managed_biguint!(min_energy_for_propose));
            })
    }

    pub fn cancel_proposal(&mut self, caller: &Address, proposal_id: usize) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                sc.cancel(proposal_id);
            })
    }

    pub fn check_proposal_id_consistency(
        &mut self,
        caller: &Address,
        proposal_id: usize,
    ) -> TxResult {
        self.b_mock
            .execute_tx(caller, &self.gov_wrapper, &rust_biguint!(0), |sc| {
                let proposal = sc.proposals().get(proposal_id);
                assert!(
                    proposal.proposal_id == proposal_id,
                    "Proposal ID is inconsistent!"
                )
            })
    }
    pub fn increment_block_nonce(&mut self, inc_amount: u64) {
        self.current_block += inc_amount;
        self.b_mock.set_block_nonce(self.current_block);
    }
}
