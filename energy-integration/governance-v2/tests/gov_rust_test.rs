mod gov_test_setup;

use multiversx_sc::{
    arrayvec::ArrayVec,
    codec::Empty,
    types::{EsdtTokenPayment, ManagedVec},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_buffer, managed_token_id, rust_biguint, DebugApi,
};
use gov_test_setup::*;
use governance_v2::{
    configurable::ConfigurablePropertiesModule,
    proposal::{FeeEntry, GovernanceAction, GovernanceProposal, ProposalFees},
    proposal_storage::ProposalStorageModule,
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
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // vote too early
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_user_error("Proposal is not active");

    gov_setup.increment_block_nonce(VOTING_DELAY_BLOCKS);

    // try execute before queue
    gov_setup
        .execute(proposal_id)
        .assert_user_error("Can only execute queued proposals");

    // try queue before voting ends
    gov_setup
        .queue(proposal_id)
        .assert_user_error("Can only queue succeeded proposals");

    gov_setup.increment_block_nonce(VOTING_PERIOD_BLOCKS);

    // try queue not enough votes
    gov_setup
        .queue(proposal_id)
        .assert_user_error("Can only queue succeeded proposals");

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
    gov_setup.queue(proposal_id).assert_ok();

    // try execute too early
    gov_setup
        .execute(proposal_id)
        .assert_user_error("Proposal is in timelock status. Try again later");

    // execute ok
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
fn gov_no_veto_vote_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeQuorum",
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
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeQuorum",
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
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_000),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        MIN_FEE_FOR_PROPOSE,
        &sc_addr,
        b"changeQuorum",
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

#[test]
fn gov_additional_payment_to_propose_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(100),
        &Empty,
    );

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(950),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        100,
        &sc_addr,
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );

    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // vote too early
    gov_setup.set_block_nonce(20);
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_user_error("Proposal is not active");

    gov_setup
        .deposit_tokens(&second_user_addr, 950, proposal_id)
        .assert_ok();

    gov_setup.b_mock.check_nft_balance::<Empty>(
        &sc_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(1_050),
        None,
    );

    // quorum is 1_500
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum().get(), managed_biguint!(1_500));
        })
        .assert_ok();

    gov_setup.set_block_nonce(20);

    // First user Up Vote
    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();

    // Second user Up Vote
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
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
fn gov_wait_for_fees_cancel_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(100),
        &Empty,
    );

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(500),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        100,
        &sc_addr,
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );

    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // vote too early
    gov_setup.set_block_nonce(20);
    gov_setup
        .deposit_tokens(&second_user_addr, 500, proposal_id)
        .assert_ok();

    // Check users don't have any funds
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    gov_setup.b_mock.check_nft_balance::<Empty>(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    // Check that SC has user funds
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &sc_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(600),
        None,
    );

    gov_setup.set_block_nonce(20);

    // Vote is not Active
    gov_setup
        .up_vote(&first_user_addr, proposal_id)
        .assert_user_error("Proposal is not active");

    // Cancel while still in state WaitForFees
    gov_setup.cancel(&second_user_addr, proposal_id).assert_ok();

    // Check funds are returned to users
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(100),
        None,
    );

    gov_setup.b_mock.check_nft_balance::<Empty>(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(500),
        None,
    );
}

#[test]
fn gov_claim_deposited_token_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(100),
        &Empty,
    );

    // Give proposer the minimum fee
    gov_setup.b_mock.set_nft_balance(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(500),
        &Empty,
    );

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        100,
        &sc_addr,
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );

    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // vote too early
    gov_setup.set_block_nonce(20);
    gov_setup
        .deposit_tokens(&second_user_addr, 500, proposal_id)
        .assert_ok();

    // Check users don't have any funds
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    gov_setup.b_mock.check_nft_balance::<Empty>(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    // Check that SC has user funds
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &sc_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(600),
        None,
    );

    gov_setup.set_block_nonce(20);

    // Vote is not Active
    gov_setup
        .up_vote(&first_user_addr, proposal_id)
        .assert_user_error("Proposal is not active");

    // Cancel while still in state WaitForFees
    gov_setup
        .claim_deposited_tokens(&second_user_addr, proposal_id)
        .assert_ok();

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let mut expected_actions = ArrayVec::new();
            expected_actions.push(GovernanceAction {
                dest_address: managed_address!(&sc_addr),
                function_name: managed_buffer!(b"changeQuorum"),
                arguments: ManagedVec::from_single_item(managed_buffer!(
                    &1_000u64.to_be_bytes()[..]
                )),
                gas_limit: GAS_LIMIT,
            });

            let fee_entry = FeeEntry {
                depositor_addr: managed_address!(&first_user_addr),
                tokens: EsdtTokenPayment::<DebugApi> {
                    token_identifier: managed_token_id!(LKMEX_TOKEN_ID),
                    token_nonce: 1,
                    amount: managed_biguint!(100),
                },
            };
            let expected_fees = ManagedVec::from_single_item(fee_entry);
            let expected_proposal = GovernanceProposal::<DebugApi> {
                proposer: managed_address!(&first_user_addr),
                description: managed_buffer!(b"change quorum"),
                actions: expected_actions,
                fees: ProposalFees {
                    total_amount: managed_biguint!(100),
                    entries: expected_fees,
                },
            };

            let actual_proposal = sc.proposals().get(proposal_id);
            assert_eq!(actual_proposal, expected_proposal);
        })
        .assert_ok();

    // Check funds are returned to second user only
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &first_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(0),
        None,
    );

    gov_setup.b_mock.check_nft_balance::<Empty>(
        &second_user_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(500),
        None,
    );

    // Check that SC has still has first user's funds
    gov_setup.b_mock.check_nft_balance::<Empty>(
        &sc_addr,
        LKMEX_TOKEN_ID,
        1,
        &rust_biguint!(100),
        None,
    );
}
