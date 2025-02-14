#![allow(deprecated)]

mod gov_test_setup;

use gov_test_setup::*;
use governance_v2::{
    configurable::ConfigurablePropertiesModule, proposal::GovernanceProposalStatus,
    proposal_storage::ProposalStorageModule, views::ViewsModule,
};
use multiversx_sc::types::ManagedVec;
use multiversx_sc_scenario::{managed_buffer, rust_biguint};

#[test]
fn init_gov_test() {
    let _ = GovSetup::new(governance_v2::contract_obj);
}

#[test]
fn gov_propose_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let second_user_addr = gov_setup.second_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
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

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_ok();

    // user 2 try vote again
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_user_error("Already voted for this proposal");

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Succeeded,
                "Action should have been Succeeded"
            );
        })
        .assert_ok();

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            let proposal = sc.proposals().get(1);
            let action = proposal.actions.first().unwrap();
            let mut args_managed = ManagedVec::new();
            args_managed.push(managed_buffer!(&1_000u64.to_be_bytes()));

            assert!(
                action.function_name == b"changeTODO",
                "Wrong Action - Endpoint Name"
            );
            assert!(action.arguments == args_managed, "Wrong Action - Arguments");
        })
        .assert_ok();
}

#[test]
fn gov_propose_total_energy_0_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let no_energy_user = gov_setup.no_energy_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&no_energy_user, MEX_TOKEN_ID, &min_fee);

    gov_setup.change_min_energy(0).assert_ok();

    let (result, proposal_id) = gov_setup.propose(
        &no_energy_user,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Pending,
                "Action should have been Defeated"
            );
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
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert_eq!(sc.quorum_percentage().get(), QUORUM_PERCENTAGE);
        })
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .up_vote(&second_user_addr, proposal_id)
        .assert_ok();

    // Third User DownWithVetoVote = 1_100
    gov_setup
        .down_veto_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::DefeatedWithVeto,
                "Action should have been Defeated"
            );
        })
        .assert_ok();
}

#[test]
fn gov_abstain_vote_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .abstain_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Defeated,
                "Action should have been Defeated"
            );
        })
        .assert_ok();
}

#[test]
fn gov_no_quorum_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Defeated,
                "Action should have been Defeated"
            );
        })
        .assert_ok();
}

#[test]
fn gov_modify_quorum_after_end_vote_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Defeated,
                "Action should have been Defeated"
            );
            sc.try_change_quorum_percentage(QUORUM_PERCENTAGE / 2);
            assert!(sc.quorum_percentage().get() == QUORUM_PERCENTAGE / 2);

            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Defeated,
                "Action should have been Defeated"
            );
        })
        .assert_ok();
}

#[test]
fn gov_withdraw_defeated_proposal_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .down_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Defeated,
                "Action should have been Defeated"
            );
        })
        .assert_ok();

    // Other user (not proposer) try to withdraw the fee -> Fail
    gov_setup
        .withdraw_after_defeated(&third_user_addr, proposal_id)
        .assert_error(4, "Only original proposer may withdraw a pending proposal");

    // Proposer withdraw
    gov_setup
        .withdraw_after_defeated(&first_user_addr, proposal_id)
        .assert_ok();

    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);
}

#[test]
fn gov_modify_withdraw_defeated_proposal_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // Check proposer balance
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .down_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .change_withdraw_percentage(FULL_PERCENTAGE + 1u64)
        .assert_error(4, "Not valid value for withdraw percentage if defeated!");

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::Defeated,
                "Action should have been Defeated"
            );

            sc.try_change_withdraw_percentage_defeated(WITHDRAW_PERCENTAGE / 5);

            assert!(sc.withdraw_percentage_defeated().get() == WITHDRAW_PERCENTAGE / 5);
        })
        .assert_ok();

    // Other user (not proposer) try to withdraw the fee -> Fail
    gov_setup
        .withdraw_after_defeated(&third_user_addr, proposal_id)
        .assert_error(4, "Only original proposer may withdraw a pending proposal");

    // Proposer withdraw
    gov_setup
        .withdraw_after_defeated(&first_user_addr, proposal_id)
        .assert_ok();

    // Check proposer balance (fee)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);
}

#[test]
fn gov_withdraw_no_with_veto_defeated_proposal_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // Check proposer balance
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .down_veto_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::DefeatedWithVeto,
                "Action should have been Defeated"
            );
        })
        .assert_ok();

    // Other user (not proposer) withdraw the fee
    gov_setup
        .withdraw_after_defeated(&third_user_addr, proposal_id)
        .assert_ok();

    // Check proposer balance (fee)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &(&min_fee / 2u64));

    // Withdraw the fee twice - error
    gov_setup
        .withdraw_after_defeated(&third_user_addr, proposal_id)
        .assert_error(4, "Fee already withdrawn!");
}

#[test]
fn gov_withdraw_no_with_veto_penalty_limits_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let third_user_addr = gov_setup.third_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    gov_setup.change_withdraw_percentage(0).assert_ok();
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);

    // Check proposer balance
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .down_veto_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::DefeatedWithVeto,
                "Action should have been Defeated"
            );
        })
        .assert_ok();

    // Other user (not proposer) withdraw the fee
    gov_setup
        .withdraw_after_defeated(&third_user_addr, proposal_id)
        .assert_ok();

    // Check proposer balance (fee)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);

    gov_setup.change_withdraw_percentage(10_000).assert_ok();
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 2);

    // Check proposer balance
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup.up_vote(&first_user_addr, proposal_id).assert_ok();
    gov_setup
        .down_veto_vote(&third_user_addr, proposal_id)
        .assert_ok();

    gov_setup.increment_timestamp(VOTING_PERIOD_SECONDS);

    gov_setup
        .b_mock
        .execute_query(&gov_setup.gov_wrapper, |sc| {
            assert!(
                sc.get_proposal_status(1) == GovernanceProposalStatus::DefeatedWithVeto,
                "Action should have been Defeated"
            );
        })
        .assert_ok();

    // Other user (not proposer) withdraw the fee
    gov_setup
        .withdraw_after_defeated(&third_user_addr, proposal_id)
        .assert_ok();

    // Check proposer balance (fee)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);
}

#[test]
fn gov_propose_cancel_proposal_id_test() {
    let mut gov_setup = GovSetup::new(governance_v2::contract_obj);

    let first_user_addr = gov_setup.first_user.clone();
    let sc_addr = gov_setup.gov_wrapper.address_ref().clone();
    let min_fee = rust_biguint!(MIN_FEE_FOR_PROPOSE) * DECIMALS_CONST;
    // Give proposer the minimum fee
    gov_setup
        .b_mock
        .set_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &(&min_fee * 3u64));

    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 1);
    gov_setup
        .check_proposal_id_consistency(&first_user_addr, proposal_id)
        .assert_ok();

    // Proposal ID = 2
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 2);
    gov_setup
        .check_proposal_id_consistency(&first_user_addr, proposal_id)
        .assert_ok();

    // Proposal ID = 3
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 3);
    gov_setup
        .check_proposal_id_consistency(&first_user_addr, proposal_id)
        .assert_ok();

    // Check proposer balance (fee = 0)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &rust_biguint!(0));

    gov_setup.cancel_proposal(&first_user_addr, 2).assert_ok();

    // Try to retrieve the cancelled proposal
    gov_setup
        .b_mock
        .execute_tx(
            &gov_setup.first_user.clone(),
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.proposals().get(2);
            },
        )
        .assert_ok();

    // Check proposer balance (fee should be refunded)
    gov_setup
        .b_mock
        .check_esdt_balance(&first_user_addr, MEX_TOKEN_ID, &min_fee);
    assert_eq!(proposal_id, 3);
    gov_setup
        .check_proposal_id_consistency(&first_user_addr, proposal_id)
        .assert_ok();

    // Proposal ID = 4
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 4);
    gov_setup
        .check_proposal_id_consistency(&first_user_addr, proposal_id)
        .assert_ok();

    gov_setup.cancel_proposal(&first_user_addr, 4).assert_ok();

    // Try to retrieve the cancelled proposal
    gov_setup
        .b_mock
        .execute_tx(
            &gov_setup.first_user.clone(),
            &gov_setup.gov_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.proposals().get(4);
            },
        )
        .assert_ok();

    // Proposal ID = 5
    let (result, proposal_id) = gov_setup.propose(
        &first_user_addr,
        &min_fee,
        &sc_addr,
        b"changeTODO",
        vec![1_000u64.to_be_bytes().to_vec()],
    );
    result.assert_ok();
    assert_eq!(proposal_id, 5);
    gov_setup
        .check_proposal_id_consistency(&first_user_addr, proposal_id)
        .assert_ok();
}
