use elrond_wasm::types::{
    Address, BigUint, EsdtLocalRole, ManagedAddress, MultiResult3, OptionalArg, TokenIdentifier,
};
use elrond_wasm_debug::tx_mock::TxInputESDT;
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    DebugApi,
};

const PAIR_WASM_PATH: &'static str = "pair/output/pair.wasm";
const WEGLD_TOKEN_ID: &[u8] = b"WEGLD-abcdef";
const RIDE_TOKEN_ID: &[u8] = b"RIDE-abcdef";
const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef";

const USER_TOTAL_WEGLD_TOKENS: u64 = 5_000_000_000;
const USER_TOTAL_RIDE_TOKENS: u64 = 5_000_000_000;

use pair::config::*;
use pair::*;

#[allow(dead_code)]
struct PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn(DebugApi) -> pair::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
}

fn setup_pair<PairObjBuilder>(pair_builder: PairObjBuilder) -> PairSetup<PairObjBuilder>
where
    PairObjBuilder: 'static + Copy + Fn(DebugApi) -> pair::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);
    let pair_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_addr),
        pair_builder,
        PAIR_WASM_PATH,
    );

    blockchain_wrapper.execute_tx(&owner_addr, &pair_wrapper, &rust_zero, |sc| {
        let first_token_id = managed_token_id!(WEGLD_TOKEN_ID);
        let second_token_id = managed_token_id!(RIDE_TOKEN_ID);
        let router_address = managed_address!(&owner_addr);
        let router_owner_address = managed_address!(&owner_addr);
        let total_fee_percent = 300u64;
        let special_fee_percent = 50u64;

        sc.init(
            first_token_id,
            second_token_id,
            router_address,
            router_owner_address,
            total_fee_percent,
            special_fee_percent,
            OptionalArg::None,
        );

        let lp_token_id = managed_token_id!(LP_TOKEN_ID);
        sc.lp_token_identifier().set(&lp_token_id);

        sc.state().set(&State::Active);

        StateChange::Commit
    });

    let lp_token_roles = [EsdtLocalRole::Mint, EsdtLocalRole::Burn];
    blockchain_wrapper.set_esdt_local_roles(
        pair_wrapper.address_ref(),
        LP_TOKEN_ID,
        &lp_token_roles[..],
    );

    let user_addr = blockchain_wrapper.create_user_account(&rust_biguint!(100_000_000));
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        WEGLD_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_WEGLD_TOKENS),
    );
    blockchain_wrapper.set_esdt_balance(
        &user_addr,
        RIDE_TOKEN_ID,
        &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
    );

    PairSetup {
        blockchain_wrapper,
        owner_address: owner_addr,
        user_address: user_addr,
        pair_wrapper,
    }
}

fn add_liquidity<PairObjBuilder>(
    pair_setup: &mut PairSetup<PairObjBuilder>,
    first_token_amount: u64,
    first_token_min: u64,
    second_token_amount: u64,
    second_token_min: u64,
    expected_lp_amount: u64,
    expected_first_amount: u64,
    expected_second_amount: u64,
) where
    PairObjBuilder: 'static + Copy + Fn(DebugApi) -> pair::ContractObj<DebugApi>,
{
    let payments = vec![
        TxInputESDT {
            token_identifier: WEGLD_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(first_token_amount),
        },
        TxInputESDT {
            token_identifier: RIDE_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(second_token_amount),
        },
    ];

    pair_setup.blockchain_wrapper.execute_esdt_multi_transfer(
        &pair_setup.user_address,
        &pair_setup.pair_wrapper,
        &payments,
        |sc| {
            let MultiResult3 { 0: payments } = sc.add_liquidity(
                managed_biguint!(first_token_min),
                managed_biguint!(second_token_min),
                OptionalArg::None,
            );

            assert_eq!(payments.0.token_identifier, managed_token_id!(LP_TOKEN_ID));
            assert_eq!(payments.0.token_nonce, 0);
            assert_eq!(payments.0.amount, managed_biguint!(expected_lp_amount));

            assert_eq!(
                payments.1.token_identifier,
                managed_token_id!(WEGLD_TOKEN_ID)
            );
            assert_eq!(payments.1.token_nonce, 0);
            assert_eq!(payments.1.amount, managed_biguint!(expected_first_amount));

            assert_eq!(
                payments.2.token_identifier,
                managed_token_id!(RIDE_TOKEN_ID)
            );
            assert_eq!(payments.2.token_nonce, 0);
            assert_eq!(payments.2.amount, managed_biguint!(expected_second_amount));

            StateChange::Commit
        },
    );
}

#[test]
fn test_add_liquidity() {
    let mut pair_setup = setup_pair(pair::contract_obj);

    add_liquidity(
        &mut pair_setup,
        1_001_000,
        1_000_000,
        1_001_000,
        1_000_000,
        1_000_000,
        1_001_000,
        1_001_000,
    );
}
