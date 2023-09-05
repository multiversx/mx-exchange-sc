#![allow(deprecated)]

use multiversx_sc::codec::multi_types::{MultiValue2, OptionalValue};
use multiversx_sc::types::MultiValueEncoded;
use multiversx_sc::types::{Address, EsdtLocalRole, EsdtTokenPayment, ManagedVec};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint,
    whitebox_legacy::*, DebugApi,
};

use governance::config::*;
use governance::errors::*;
use governance::proposal::*;
use governance::vote::*;
use governance::*;

use pair_mock::*;

pub const GOVERNANCE_WASM_PATH: &str = "governance/output/governance.wasm";
pub const PAIR_MOCK_WASM_PATH: &str = "pair-mock/output/pair-mock.wasm";
pub const VOTE_NFT_ID: &[u8] = b"VOTE-abcdef";
pub const FAKE_TOKEN_ID: &[u8] = b"FAKE-abcdef";
pub const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
pub const LPMEX_TOKEN_ID: &[u8] = b"LPMEX-abcdef";
pub const WUSDC_TOKEN_ID: &[u8] = b"WUSDC-abcdef";
pub const USER_TOTAL_MEX_TOKENS: u64 = 5_000_000_000;
pub const QUORUM: u64 = 1_000_000_000;
pub const VOTING_DELAY_IN_BLOCKS: u64 = 1;
pub const VOTING_PERIOD_IN_BLOCKS: u64 = 1;
pub const MIN_WEIGHT_FOR_PROPOSAL: u64 = 1_000_000;

pub struct GovernanceSetup<GovernanceObjBuilder, PairMockObjBuilder>
where
    GovernanceObjBuilder: 'static + Copy + Fn() -> governance::ContractObj<DebugApi>,
    PairMockObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub gov_wrapper: ContractObjWrapper<governance::ContractObj<DebugApi>, GovernanceObjBuilder>,
    pub pair_wrapper: ContractObjWrapper<pair_mock::ContractObj<DebugApi>, PairMockObjBuilder>,
}

pub fn setup_gov<GovernanceObjBuilder, PairMockObjBuilder>(
    gov_builder: GovernanceObjBuilder,
    pair_builder: PairMockObjBuilder,
) -> GovernanceSetup<GovernanceObjBuilder, PairMockObjBuilder>
where
    GovernanceObjBuilder: 'static + Copy + Fn() -> governance::ContractObj<DebugApi>,
    PairMockObjBuilder: 'static + Copy + Fn() -> pair_mock::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);
    let gov_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        gov_builder,
        GOVERNANCE_WASM_PATH,
    );

    let pair_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        pair_builder,
        PAIR_MOCK_WASM_PATH,
    );

    // init DEX mock
    blockchain_wrapper
        .execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
            sc.init(
                OptionalValue::Some(managed_token_id!(MEX_TOKEN_ID)),
                OptionalValue::Some(managed_token_id!(WUSDC_TOKEN_ID)),
                OptionalValue::None,
                OptionalValue::None,
                OptionalValue::None,
                OptionalValue::None,
            );
        })
        .assert_ok();

    blockchain_wrapper
        .execute_tx(&owner_addr, &gov_wrapper, &rust_zero, |sc| {
            let mut price_providers = MultiValueEncoded::new();
            price_providers.push(MultiValue2::from((
                managed_token_id!(LPMEX_TOKEN_ID),
                managed_address!(pair_wrapper.address_ref()),
            )));

            sc.init(
                managed_biguint!(QUORUM),
                VOTING_DELAY_IN_BLOCKS,
                VOTING_PERIOD_IN_BLOCKS,
                managed_token_id!(VOTE_NFT_ID),
                managed_token_id!(MEX_TOKEN_ID),
                managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
                ManagedVec::from(vec![
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_token_id!(LPMEX_TOKEN_ID),
                ]),
                price_providers,
            );
        })
        .assert_ok();

    let vote_nft_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftBurn,
        EsdtLocalRole::NftUpdateAttributes,
    ];
    blockchain_wrapper.set_esdt_local_roles(
        gov_wrapper.address_ref(),
        VOTE_NFT_ID,
        &vote_nft_roles[..],
    );

    let user_addr = blockchain_wrapper.create_user_account(&rust_biguint!(100_000_000));
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        MEX_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &owner_addr,
        MEX_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        LPMEX_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &owner_addr,
        LPMEX_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        FAKE_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &owner_addr,
        FAKE_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );

    GovernanceSetup {
        blockchain_wrapper,
        owner_address: owner_addr,
        user_address: user_addr,
        gov_wrapper,
        pair_wrapper,
    }
}

#[test]
fn test_gov_setup() {
    let _ = setup_gov(governance::contract_obj, pair_mock::contract_obj);
}

#[test]
fn test_propose_bad_token() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            FAKE_TOKEN_ID,
            0,
            &rust_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
            |sc| {
                sc.propose(ProposalCreationArgs {
                    description: managed_buffer!(&b""[..]),
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                });
            },
        )
        .assert_user_error(&String::from_utf8(UNREGISTERED_TOKEN_ID.to_vec()).unwrap());
}

#[test]
fn test_propose_bad_amount() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(MIN_WEIGHT_FOR_PROPOSAL - 1),
            |sc| {
                sc.propose(ProposalCreationArgs {
                    description: managed_buffer!(&b""[..]),
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                });
            },
        )
        .assert_user_error(&String::from_utf8(NOT_ENOUGH_FUNDS_TO_PROPOSE.to_vec()).unwrap());
}

#[test]
fn test_basic_propose() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    // User makes a basic proposal
    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
            |sc| {
                sc.propose(ProposalCreationArgs {
                    description: managed_buffer!(&b""[..]),
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                });
            },
        )
        .assert_ok();

    // Owner has to have its Vote NFT
    let owner_address = gov_setup.owner_address.clone();
    gov_setup
        .blockchain_wrapper
        .execute_in_managed_environment(|| {
            gov_setup.blockchain_wrapper.check_nft_balance(
                &gov_setup.owner_address,
                VOTE_NFT_ID,
                1,
                &rust_biguint!(1),
                Some(&VoteNFTAttributes::<DebugApi> {
                    proposal_id: 0,
                    vote_type: VoteType::Upvote,
                    vote_weight: managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
                    voter: managed_address!(&owner_address),
                    payment: EsdtTokenPayment::new(
                        managed_token_id!(MEX_TOKEN_ID),
                        0,
                        managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
                    ),
                }),
            );
        });

    // SC Storage for proposal should be set correctly
    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let proposal = sc.proposal(0).get();

            assert_eq!(0, proposal.id,);
            assert_eq!(0, proposal.creation_block);
            assert_eq!(managed_address!(&owner_address), proposal.proposer);
            assert_eq!(0, proposal.description.len());
            assert!(!proposal.was_executed);
            assert_eq!(0, proposal.actions.len());
            assert_eq!(
                managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
                proposal.num_upvotes,
            );
            assert_eq!(managed_biguint!(0), proposal.num_downvotes);

            assert_eq!(1, sc.proposal_id_counter().get());
        })
        .assert_ok();
}

#[test]
fn test_proposal_status_change() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    gov_setup.blockchain_wrapper.set_block_nonce(0);

    gov_setup
        .blockchain_wrapper
        .execute_tx(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                let dummy_proposal = Proposal::<DebugApi> {
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                    creation_block: 0,
                    description: managed_buffer!(&[]),
                    id: 0,
                    num_downvotes: managed_biguint!(0),
                    num_upvotes: managed_biguint!(0),
                    proposer: managed_address!(&Address::zero()),
                    was_executed: false,
                };

                sc.proposal(0).set(dummy_proposal);
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Pending, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .set_block_nonce(VOTING_DELAY_IN_BLOCKS);

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Active, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .set_block_nonce(VOTING_DELAY_IN_BLOCKS + VOTING_PERIOD_IN_BLOCKS);

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Defeated, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_tx(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                let dummy_proposal = Proposal::<DebugApi> {
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                    creation_block: 0,
                    description: managed_buffer!(&[]),
                    id: 0,
                    num_downvotes: managed_biguint!(0),
                    num_upvotes: managed_biguint!(QUORUM),
                    proposer: managed_address!(&Address::zero()),
                    was_executed: false,
                };

                sc.proposal(0).set(dummy_proposal);
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Succeeded, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_tx(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.execute(0);
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Executed, status);
        })
        .assert_ok();
}

#[test]
fn test_basic_reclaim() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
            |sc| {
                sc.propose(ProposalCreationArgs {
                    description: managed_buffer!(&b""[..]),
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                });
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            VOTE_NFT_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                sc.redeem();
            },
        )
        .assert_user_error(&String::from_utf8(VOTING_PERIOD_NOT_ENDED.to_vec()).unwrap());

    gov_setup
        .blockchain_wrapper
        .set_block_nonce(VOTING_DELAY_IN_BLOCKS + VOTING_PERIOD_IN_BLOCKS);

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            VOTE_NFT_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                sc.redeem();
            },
        )
        .assert_ok();

    let owner_address = gov_setup.owner_address.clone();
    gov_setup.blockchain_wrapper.check_esdt_balance(
        &owner_address,
        MEX_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_MEX_TOKENS),
    );
}

#[test]
fn test_vote() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    gov_setup.blockchain_wrapper.set_block_nonce(0);

    gov_setup
        .blockchain_wrapper
        .execute_tx(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                let dummy_proposal = Proposal::<DebugApi> {
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                    creation_block: 0,
                    description: managed_buffer!(&[]),
                    id: 0,
                    num_downvotes: managed_biguint!(0),
                    num_upvotes: managed_biguint!(0),
                    proposer: managed_address!(&Address::zero()),
                    was_executed: false,
                };

                sc.proposal(0).set(dummy_proposal);
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Pending, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(101),
            |sc| {
                sc.upvote(0);
            },
        )
        .assert_user_error(&String::from_utf8(PROPOSAL_NOT_ACTIVE.to_vec()).unwrap());

    gov_setup
        .blockchain_wrapper
        .set_block_nonce(VOTING_DELAY_IN_BLOCKS);

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Active, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(101),
            |sc| {
                sc.upvote(0);
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            MEX_TOKEN_ID,
            0,
            &rust_biguint!(102),
            |sc| {
                sc.downvote(0);
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let proposal = sc.proposal(0).get();

            assert_eq!(0, proposal.id,);
            assert_eq!(0, proposal.creation_block);
            assert_eq!(managed_address!(&Address::zero()), proposal.proposer);
            assert_eq!(0, proposal.description.len());
            assert!(!proposal.was_executed);
            assert_eq!(0, proposal.actions.len());
            assert_eq!(managed_biguint!(101), proposal.num_upvotes,);
            assert_eq!(managed_biguint!(102), proposal.num_downvotes);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            VOTE_NFT_ID,
            2,
            &rust_biguint!(1),
            |sc| {
                sc.redeem();
            },
        )
        .assert_user_error(&String::from_utf8(VOTING_PERIOD_NOT_ENDED.to_vec()).unwrap());

    gov_setup
        .blockchain_wrapper
        .set_block_nonce(VOTING_DELAY_IN_BLOCKS + VOTING_PERIOD_IN_BLOCKS);

    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let status = sc.get_proposal_status_view(0);
            assert_eq!(ProposalStatus::Defeated, status);
        })
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            VOTE_NFT_ID,
            1,
            &rust_biguint!(1),
            |sc| {
                sc.redeem();
            },
        )
        .assert_ok();

    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            VOTE_NFT_ID,
            2,
            &rust_biguint!(1),
            |sc| {
                sc.redeem();
            },
        )
        .assert_ok();
}

#[test]
fn test_basic_propose_with_lpmex() {
    let mut gov_setup = setup_gov(governance::contract_obj, pair_mock::contract_obj);

    // User makes a basic proposal
    gov_setup
        .blockchain_wrapper
        .execute_esdt_transfer(
            &gov_setup.owner_address,
            &gov_setup.gov_wrapper,
            LPMEX_TOKEN_ID,
            0,
            &rust_biguint!(MIN_WEIGHT_FOR_PROPOSAL * 2),
            |sc| {
                sc.propose(ProposalCreationArgs {
                    description: managed_buffer!(&b""[..]),
                    actions: ManagedVec::from(Vec::<Action<DebugApi>>::new()),
                });
            },
        )
        .assert_ok();

    // Owner has to have its Vote NFT
    let owner_address = gov_setup.owner_address.clone();
    gov_setup
        .blockchain_wrapper
        .execute_in_managed_environment(|| {
            gov_setup.blockchain_wrapper.check_nft_balance(
                &gov_setup.owner_address,
                VOTE_NFT_ID,
                1,
                &rust_biguint!(1),
                Some(&VoteNFTAttributes::<DebugApi> {
                    proposal_id: 0,
                    vote_type: VoteType::Upvote,
                    vote_weight: managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
                    voter: managed_address!(&owner_address),
                    payment: EsdtTokenPayment::new(
                        managed_token_id!(LPMEX_TOKEN_ID),
                        0,
                        managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL * 2),
                    ),
                }),
            );
        });

    // SC Storage for proposal should be set correctly
    gov_setup
        .blockchain_wrapper
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let proposal = sc.proposal(0).get();

            assert_eq!(0, proposal.id,);
            assert_eq!(0, proposal.creation_block);
            assert_eq!(managed_address!(&owner_address), proposal.proposer);
            assert_eq!(0, proposal.description.len());
            assert!(!proposal.was_executed);
            assert_eq!(0, proposal.actions.len());
            assert_eq!(
                managed_biguint!(MIN_WEIGHT_FOR_PROPOSAL),
                proposal.num_upvotes,
            );
            assert_eq!(managed_biguint!(0), proposal.num_downvotes);

            assert_eq!(1, sc.proposal_id_counter().get());
        })
        .assert_ok();
}
