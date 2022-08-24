use elrond_wasm::types::Address;
use elrond_wasm_debug::{
    rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

static REWARD_TOKEN_ID: &[u8] = b"REW-123456";
static FARMING_TOKEN_ID: &[u8] = b"LPTOK-123456";
static FARM_TOKEN_ID: &[u8] = b"FARM-123456";
const DIV_SAFETY: u64 = 1_000_000_000_000_000_000;
const FARMING_TOKEN_BALANCE: u64 = 1_000_000;

#[test]
fn farm_with_no_boost_test() {}

pub struct FarmSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub last_farm_token_nonce: usize,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory_mock::ContractObj<DebugApi>, EnergyFactoryBuilder>,
}

impl<FarmObjBuilder, EnergyFactoryBuilder> FarmSetup<FarmObjBuilder, EnergyFactoryBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
{
    pub fn new(farm_builder: FarmObjBuilder, energy_factory_builder: EnergyFactoryBuilder) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let farm_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), farm_builder, "farm.wasm");
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_builder,
            "energy_factory.wasm",
        );

        FarmSetup {
            b_mock,
            owner,
            first_user,
            second_user,
            last_farm_token_nonce: 0,
            farm_wrapper,
            energy_factory_wrapper,
        }
    }
}
