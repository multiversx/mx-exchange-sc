use common_structs::Timestamp;
use config::ConfigModule;
use farm_token::FarmTokenModule;
use farm_with_top_up::FarmWithTopUp;
use multiversx_sc::{
    imports::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    imports::{BlockchainStateWrapper, ContractObjWrapper},
    managed_address, managed_biguint, managed_token_id, rust_biguint, DebugApi,
};
use pausable::{PausableModule, State};
use permissions_hub::PermissionsHub;
use permissions_hub_module::PermissionsHubModule;
use timestamp_oracle::{epoch_to_timestamp::EpochToTimestampModule, TimestampOracle};

pub static REWARD_TOKEN_ID: &[u8] = b"REW-123456";
pub static FARMING_TOKEN_ID: &[u8] = b"LPTOK-123456";
pub static FARM_TOKEN_ID: &[u8] = b"FARM-123456";
pub const DIV_SAFETY: u64 = 1_000_000_000_000;
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 1_000;
pub const FARMING_TOKEN_BALANCE: u64 = 200_000_000;
pub const TIMESTAMP_PER_EPOCH: Timestamp = 24 * 60 * 60;

pub struct FarmWithTopUpSetup<FarmObjBuilder, TimestampOracleObjBuilder, PermissionsHubObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_top_up::ContractObj<DebugApi>,
    TimestampOracleObjBuilder: 'static + Copy + Fn() -> timestamp_oracle::ContractObj<DebugApi>,
    PermissionsHubObjBuilder: 'static + Copy + Fn() -> permissions_hub::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub user: Address,
    pub farm_wrapper: ContractObjWrapper<farm_with_top_up::ContractObj<DebugApi>, FarmObjBuilder>,
    pub timestamp_oracle_wrapper:
        ContractObjWrapper<timestamp_oracle::ContractObj<DebugApi>, TimestampOracleObjBuilder>,
    pub permissions_hub_wrapper:
        ContractObjWrapper<permissions_hub::ContractObj<DebugApi>, PermissionsHubObjBuilder>,
}

impl<FarmObjBuilder, TimestampOracleObjBuilder, PermissionsHubObjBuilder>
    FarmWithTopUpSetup<FarmObjBuilder, TimestampOracleObjBuilder, PermissionsHubObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_top_up::ContractObj<DebugApi>,
    TimestampOracleObjBuilder: 'static + Copy + Fn() -> timestamp_oracle::ContractObj<DebugApi>,
    PermissionsHubObjBuilder: 'static + Copy + Fn() -> permissions_hub::ContractObj<DebugApi>,
{
    pub fn new(
        farm_builder: FarmObjBuilder,
        timestamp_oracle_builder: TimestampOracleObjBuilder,
        permissions_hub_builder: PermissionsHubObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let user = b_mock.create_user_account(&rust_zero);
        let farm_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), farm_builder, "farm.wasm");

        let timestamp_oracle_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            timestamp_oracle_builder,
            "timestamp oracle",
        );
        b_mock
            .execute_tx(&owner, &timestamp_oracle_wrapper, &rust_zero, |sc| {
                sc.init(0);

                for i in 0..=100 {
                    sc.set_start_timestamp_for_epoch(i, i * TIMESTAMP_PER_EPOCH + 1);
                }
            })
            .assert_ok();

        let permissions_hub_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            permissions_hub_builder,
            "permissions_hub.wasm",
        );

        b_mock
            .execute_tx(&owner, &permissions_hub_wrapper, &rust_zero, |sc| {
                sc.init();
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &farm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(REWARD_TOKEN_ID);
                let farming_token_id = managed_token_id!(FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIV_SAFETY);

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    managed_address!(&owner),
                    managed_address!(timestamp_oracle_wrapper.address_ref()),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

                sc.per_block_reward_amount()
                    .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);

                sc.set_permissions_hub_address(managed_address!(
                    permissions_hub_wrapper.address_ref()
                ));
            })
            .assert_ok();

        let farm_token_roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];
        b_mock.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            FARM_TOKEN_ID,
            &farm_token_roles[..],
        );

        let farming_token_roles = [EsdtLocalRole::Burn];
        b_mock.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            FARMING_TOKEN_ID,
            &farming_token_roles[..],
        );

        let reward_token_roles = [EsdtLocalRole::Mint];
        b_mock.set_esdt_local_roles(
            farm_wrapper.address_ref(),
            REWARD_TOKEN_ID,
            &reward_token_roles[..],
        );

        b_mock.set_esdt_balance(
            &user,
            FARMING_TOKEN_ID,
            &rust_biguint!(FARMING_TOKEN_BALANCE),
        );

        FarmWithTopUpSetup {
            b_mock,
            owner,
            user,
            farm_wrapper,
            timestamp_oracle_wrapper,
            permissions_hub_wrapper,
        }
    }
}
