mod gov_test_setup;

use elrond_wasm_debug::{managed_biguint, rust_biguint};
use gov_test_setup::*;
use governance_v2::{
    configurable::ConfigurablePropertiesModule, proposal_storage::ProposalStorageModule,
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
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &sc_addr,
        Vec::new(),
        b"changeQuorum",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // vote too early
    gov_setup
        .vote(&second_user_addr, proposal_id)
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
    gov_setup.vote(&second_user_addr, proposal_id).assert_ok();

    // user 2 try vote again
    gov_setup
        .vote(&second_user_addr, proposal_id)
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
fn gov_cancel_defeated_proposal_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let (result, proposal_id) = gov_setup.propose(
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
        .downvote(&second_user_addr, proposal_id)
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
