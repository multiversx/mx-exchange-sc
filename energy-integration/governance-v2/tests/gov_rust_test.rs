mod gov_test_setup;

use elrond_wasm::types::ManagedAddress;
use elrond_wasm_debug::{managed_biguint, rust_biguint, DebugApi};
use gov_test_setup::*;
use governance_v2::{
    configurable::ConfigurablePropertiesModule, proposal_storage::ProposalStorageModule, views::ViewsModule,
};

#[test]
fn init_gov_test() {
    let _ = GovSetup::new(governance_v2::contract_obj);
}

#[test]
fn test_user_voted() {
    let _ = DebugApi::dummy();
    
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let first_user_addr = gov_setup.first_merkle_user.clone();
    let first_user_power = gov_setup.get_first_user_voting_power();
    let first_user_proof = gov_setup.first_merkle_proof();

     // Unexisting
     gov_setup
     .b_mock
     .execute_query(&gov_setup.gov_wrapper, |sc| {
         let ma: ManagedAddress<DebugApi> = ManagedAddress::from_address(&first_user_addr);
         assert_eq!(sc.user_voted_proposal(1, ma), false);
     })
     .assert_ok();

    let (result, proposal_id) = gov_setup.propose(
        gov_setup.get_merkle_root_hash(),
        &first_user_addr,
        &sc_addr,
        Vec::new(),
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);
    gov_setup.set_block_nonce(20);

   // Before the vote
   gov_setup
   .b_mock
   .execute_query(&gov_setup.gov_wrapper, |sc| {
       let ma: ManagedAddress<DebugApi> = ManagedAddress::from_address(&first_user_addr);
       assert_eq!(sc.user_voted_proposal(proposal_id, ma), false);
   })
   .assert_ok();

    gov_setup
        .up_vote(&first_user_addr, &first_user_power, &first_user_proof, proposal_id)
        .assert_ok();

    // After the vote
     gov_setup
     .b_mock
     .execute_query(&gov_setup.gov_wrapper, |sc| {
         let ma: ManagedAddress<DebugApi> = ManagedAddress::from_address(&first_user_addr);
         assert_eq!(sc.user_voted_proposal(1, ma), true);
     })
     .assert_ok();
}

#[test]
fn change_gov_config_test() {
    let _ = DebugApi::dummy();
    
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let voter_addr = gov_setup.first_merkle_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let (result, proposal_id) = gov_setup.propose(
        gov_setup.get_merkle_root_hash(),
        &first_user_addr,
        &sc_addr,
        Vec::new(),
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    let first_user_power = gov_setup.get_first_user_voting_power();
    let first_user_proof = gov_setup.first_merkle_proof();

    // vote too early
    gov_setup
        .up_vote(&voter_addr, &first_user_power, &first_user_proof, proposal_id)
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
        .up_vote(&voter_addr, &first_user_power, &first_user_proof, proposal_id)
        .assert_ok();

    // user 2 try vote again
    gov_setup
        .up_vote(&voter_addr, &first_user_power, &first_user_proof, proposal_id)
        .assert_user_error("Already voted for this proposal");

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
    let _ = DebugApi::dummy();
    
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_merkle_user.clone();
    let third_user_addr = gov_setup.third_merkle_user.clone();

    let voter_addr = gov_setup.first_merkle_user.clone();
    let first_user_power = gov_setup.get_first_user_voting_power();
    let first_user_proof = gov_setup.first_merkle_proof();
    let third_user_power = gov_setup.get_third_user_voting_power();
    let third_user_proof = gov_setup.third_merkle_proof();

    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let (result, proposal_id) = gov_setup.propose(
        gov_setup.get_merkle_root_hash(),
        &first_user_addr,
        &sc_addr,
        Vec::new(),
        b"changeQuorum",
        vec![217_433_990_694u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // quorum is 217_433_990_694
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum().get(), managed_biguint!(217_433_990_694));
        })
        .assert_ok();

    gov_setup.set_block_nonce(20);

    // Third user Up Vote
    gov_setup
        .up_vote(&third_user_addr, &third_user_power, &third_user_proof, proposal_id)
        .assert_ok();

    // First User DownWithVetoVote
    gov_setup.down_veto_vote(&voter_addr, &first_user_power, &first_user_proof, proposal_id).assert_ok();

    // queue Vote failed: 217433990694 DownVetoVotes > (217433990694+40000000000 TotalVotes / 3)
    gov_setup.set_block_nonce(45);
    gov_setup.queue(proposal_id).assert_user_error("Can only queue succeeded proposals");
}

#[test]
fn gov_abstain_vote_test() {
    let _ = DebugApi::dummy();

    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_merkle_user.clone();
    let first_user_power = gov_setup.get_first_user_voting_power();
    let first_user_proof = gov_setup.first_merkle_proof();
    let second_user_addr = gov_setup.second_merkle_user.clone();
    let second_user_power = gov_setup.get_second_user_voting_power();
    let second_user_proof = gov_setup.second_merkle_proof();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let (result, proposal_id) = gov_setup.propose(
        gov_setup.get_merkle_root_hash(),
        &first_user_addr,
        &sc_addr,
        Vec::new(),
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // quorum is 217_433_990_694
    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum().get(), managed_biguint!(217_433_990_694));
        })
        .assert_ok();

    gov_setup.set_block_nonce(20);

    // First user Up Vote
    gov_setup.up_vote(&first_user_addr, &first_user_power, &first_user_proof, proposal_id)
        .assert_ok();
    // Second user Abstain Vote
    gov_setup
        .abstain_vote(&second_user_addr, &second_user_power, &second_user_proof, proposal_id)
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
    let _ = DebugApi::dummy();

    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_merkle_user.clone();
    let second_user_addr = gov_setup.second_merkle_user.clone();
    let second_user_power = gov_setup.get_second_user_voting_power();
    let second_user_proof = gov_setup.second_merkle_proof();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let (result, proposal_id) = gov_setup.propose(
        gov_setup.get_merkle_root_hash(),
        &first_user_addr,
        &sc_addr,
        Vec::new(),
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup.increment_block_nonce(VOTING_DELAY_BLOCKS);
    gov_setup
        .down_vote(&second_user_addr, &second_user_power, &second_user_proof, proposal_id)
        .assert_ok();

    // try cancel too early
    gov_setup
        .cancel(&second_user_addr, proposal_id)
        .assert_user_error("Action may not be cancelled");

    gov_setup.increment_block_nonce(VOTING_PERIOD_BLOCKS);
    gov_setup.cancel(&second_user_addr, proposal_id).assert_ok();
}

#[test]
fn gov_paymnts_refund_test() {
    let _ = DebugApi::dummy();
    
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let user_addr = gov_setup.first_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();

    let token_id = b"COOL-123456".to_vec();
    let amount = 10064;
    gov_setup
        .b_mock
        .set_esdt_balance(&user_addr, &token_id, &rust_biguint!(amount));

    let payments = vec![Payment {
        token: token_id.clone(),
        nonce: 0,
        amount,
    }];
    let (result, proposal_id) = gov_setup.propose(
        gov_setup.get_merkle_root_hash(),
        &user_addr,
        &sc_addr,
        payments.clone(),
        b"giveMeTokens",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup
        .deposit_tokens(&user_addr, &payments, proposal_id)
        .assert_ok();

    gov_setup
        .b_mock
        .check_esdt_balance(&sc_addr, &token_id, &rust_biguint!(amount));

    gov_setup.increment_block_nonce(VOTING_DELAY_BLOCKS);
    gov_setup.increment_block_nonce(VOTING_PERIOD_BLOCKS);
    gov_setup.cancel(&user_addr, proposal_id).assert_ok();

    // tokens were refunded to the user
    gov_setup
        .b_mock
        .check_esdt_balance(&user_addr, &token_id, &rust_biguint!(amount));
}
