use elrond_wasm::types::{
    Address, BigUint, EsdtLocalRole, ManagedAddress, OptionalArg, SCResult, TokenIdentifier,
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    DebugApi,
};

use farm::config::*;
use farm::*;

const GENERATED_FILE_PREFIX: &'static str = "_generated_";
const MANDOS_FILE_EXTENSION: &'static str = ".scen.json";
const FARM_WASM_PATH: &'static str = "farm/output/farm.wasm";

const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID
const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // farming token ID
const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const MIN_FARMING_EPOCHS: u8 = 2;
const PENALTY_PERCENT: u64 = 10;
const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;

struct FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    pub wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
}

impl<FarmObjBuilder> FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    pub fn into_fields(
        self,
    ) -> (
        BlockchainStateWrapper,
        Address,
        Address,
        ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    ) {
        (
            self.wrapper,
            self.owner_address,
            self.user_address,
            self.farm_wrapper,
        )
    }
}

fn setup_farm<FarmObjBuilder>(farm_builder: FarmObjBuilder) -> FarmSetup<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut wrapper = BlockchainStateWrapper::new();
    let owner_addr = wrapper.create_user_account(&rust_zero);
    let farm_wrapper =
        wrapper.create_sc_account(&rust_zero, Some(&owner_addr), farm_builder, FARM_WASM_PATH);

    // init farm contract

    wrapper.execute_tx(&owner_addr, &farm_wrapper, &rust_zero, |sc| {
        let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
        let farming_token_id = managed_token_id!(LP_TOKEN_ID);
        let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
        let pair_address = managed_address!(&Address::zero());

        let result = sc.init(
            reward_token_id,
            farming_token_id,
            division_safety_constant,
            pair_address,
        );
        assert_eq!(result, SCResult::Ok(()));

        let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
        sc.farm_token_id().set(&farm_token_id);

        sc.per_block_reward_amount()
            .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));
        sc.minimum_farming_epochs().set(&MIN_FARMING_EPOCHS);
        sc.penalty_percent().set(&PENALTY_PERCENT);

        sc.state().set(&State::Active);
        sc.produce_rewards_enabled().set(&true);

        StateChange::Commit
    });

    let farm_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        FARM_TOKEN_ID,
        &farm_token_roles[..],
    );

    let farming_token_roles = [EsdtLocalRole::Burn];
    wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        LP_TOKEN_ID,
        &farming_token_roles[..],
    );

    let reward_token_roles = [EsdtLocalRole::Mint];
    wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &reward_token_roles[..],
    );

    let user_addr = wrapper.create_user_account(&rust_biguint!(100_000_000));
    wrapper.set_esdt_balance(&user_addr, LP_TOKEN_ID, &rust_biguint!(5_000_000_000));

    FarmSetup {
        wrapper,
        owner_address: owner_addr,
        user_address: user_addr,
        farm_wrapper,
    }
}

fn enter_farm<FarmObjBuilder>(farm_setup: &mut FarmSetup<FarmObjBuilder>)
where
    FarmObjBuilder: 'static + Copy + Fn(DebugApi) -> farm::ContractObj<DebugApi>,
{
    let farm_in_amount = 100_000_000u64;
    farm_setup.wrapper.execute_esdt_transfer(
        &farm_setup.user_address,
        &farm_setup.farm_wrapper,
        LP_TOKEN_ID,
        0,
        &rust_biguint!(farm_in_amount),
        |sc| {
            let result = sc.enter_farm(OptionalArg::None);
            match result {
                SCResult::Ok(payment) => {
                    assert_eq!(payment.token_identifier, managed_token_id!(FARM_TOKEN_ID));
                    assert_eq!(payment.token_nonce, 1);
                    assert_eq!(payment.amount, managed_biguint!(farm_in_amount))
                }
                SCResult::Err(err) => {
                    let err_str = String::from_utf8(err.as_bytes().to_vec()).unwrap();
                    panic!("{:?}", err_str);
                }
            }

            StateChange::Commit
        },
    );
}

fn create_generated_mandos_file_name(suffix: &str) -> String {
    let mut path = GENERATED_FILE_PREFIX.to_owned();
    path += suffix;
    path += MANDOS_FILE_EXTENSION;

    path
}

#[test]
fn test_farm_setup() {
    let (wrapper, _, _, _) = setup_farm(farm::contract_obj).into_fields();
    let file_name = create_generated_mandos_file_name("init");

    wrapper.write_mandos_output(&file_name);
}

#[test]
fn test_enter_farm() {
    let mut farm_setup = setup_farm(farm::contract_obj);
    let _ = enter_farm(&mut farm_setup);
}
