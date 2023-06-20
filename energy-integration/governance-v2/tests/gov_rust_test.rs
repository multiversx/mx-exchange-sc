mod gov_test_setup;

use gov_test_setup::*;
use governance_v2::{
    configurable::ConfigurablePropertiesModule,
    proposal::{FeeEntry, GovernanceAction, GovernanceProposal, ProposalFees},
    proposal_storage::ProposalStorageModule,
};
use multiversx_sc::{
    arrayvec::ArrayVec,
    codec::Empty,
    types::{EsdtTokenPayment, ManagedVec},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint, DebugApi,
};

#[test]
fn init_gov_test() {
    let _ = GovSetup::new(governance_v2::contract_obj);
}

#[test]
fn change_gov_config_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        WXMEX_TOKEN_ID,
        1,
        &rust_biguint!(MIN_FEE_FOR_PROPOSE),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // vote too early
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_user_error("Proposal is not active");

    gov_setup.increment_block_nonce(VOTING_PERIOD_BLOCKS);

    // user 2 vote
    gov_setup.set_block_nonce(20);
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_ok();

    // user 2 try vote again
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_user_error("Already voted for this proposal");

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    // queue ok
    gov_setup.set_block_nonce(45);

    // execute ok
    gov_setup.increment_block_nonce(LOCKING_PERIOD_BLOCKS);

    // after execution, quorum changed from 1_500 to the proposed 1_000
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let action = sc.proposals().get(1).actions.get(0).unwrap();
            assert!(action.function_name == b"changeTODO", "Wrong Action - Endpoint Name");
            assert!(action.arguments == vec![1_000u64.to_be_bytes().to_vec()], "Wrong Action - Arguments");

        })
        .assert_ok();
}

#[test]
fn gov_no_veto_vote_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        WXMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // quorum is 1_500
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum().get(), managed_biguint!(1_500));
        })
        .assert_ok();

    gov_setup.set_block_nonce(20);

    // First user Up Vote
    // Second User Up Vote
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_ok();

    // Third User DownWithVetoVote
    gov_setup
        .down_veto_vote(&third_user_addr, proposal_id)
        .assert_ok();

    // queue Vote failed: 1001 DownVetoVotes > (3001 TotalVotes / 3)
    gov_setup.set_block_nonce(45);
    gov_setup
        .queue(proposal_id)
        .assert_user_error("Can only queue succeeded proposals");
}

#[test]
fn gov_abstain_vote_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        WXMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // quorum is 1_500
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum().get(), managed_biguint!(1_500));
        })
        .assert_ok();

    gov_setup.set_block_nonce(20);

    // First user Up Vote
    // Second user Abstain Vote
    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();

    gov_setup
        .abstain_vote(&second_user_addr, proposal_id)
        .assert_ok();

    // queue: Vote passed: 1000 UP, 0 down, 0 DownVeto, 1000 Abstain
    gov_setup.set_block_nonce(45);
    gov_setup.queue(proposal_id).assert_ok();

    // execute
    gov_setup.increment_block_nonce(LOCKING_PERIOD_BLOCKS);
    gov_setup.execute(proposal_id).assert_ok();

    // after execution, quorum changed from 1_500 to the proposed 1_000
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum().get(), managed_biguint!(1_000));
            assert!(sc.proposals().item_is_empty(1));
        })
        .assert_ok();
}

#[test]
fn gov_cancel_defeated_proposal_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        WXMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup.increment_block_nonce(VOTING_DELAY_BLOCKS);
    gov_setup
        .down_vote(&second_user_addr, proposal_id)
        .assert_ok();

    // try cancel too early
    gov_setup
        .cancel(&second_user_addr, proposal_id)
        .assert_user_error("Action may not be cancelled");

    gov_setup.increment_block_nonce(VOTING_PERIOD_BLOCKS);
    gov_setup.cancel(&second_user_addr, proposal_id).assert_ok();
}
