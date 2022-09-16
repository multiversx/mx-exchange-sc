use elrond_wasm::{
    storage::mappers::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};
use simple_lock::locked_token::LockedTokenModule;
use simple_lock_energy::SimpleLockEnergy;

mod fees_collector_mock;
use fees_collector_mock::*;

pub const EPOCHS_IN_YEAR: u64 = 365;

pub static BASE_ASSET_TOKEN_ID: &[u8] = b"MEX-123456";
pub static LOCKED_TOKEN_ID: &[u8] = b"LOCKED-123456";

pub const MIN_PENALTY_PERCENTAGE: u16 = 1; // 0.01%
pub const MAX_PENALTY_PERCENTAGE: u16 = 10_000; // 100%
pub const FEES_BURN_PERCENTAGE: u16 = 5_000; // 50%
pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 5 * EPOCHS_IN_YEAR, 10 * EPOCHS_IN_YEAR]; // 1, 5 or 10 years

pub struct SimpleLockEnergySetup<ScBuilder>
where
    ScBuilder: 'static + Copy + Fn() -> simple_lock_energy::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub sc_wrapper: ContractObjWrapper<simple_lock_energy::ContractObj<DebugApi>, ScBuilder>,
    pub fees_collector_mock: Address,
}

impl<ScBuilder> SimpleLockEnergySetup<ScBuilder>
where
    ScBuilder: 'static + Copy + Fn() -> simple_lock_energy::ContractObj<DebugApi>,
{
    pub fn new(sc_builder: ScBuilder) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let sc_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), sc_builder, "simple lock energy");
        let fees_collector_mock = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            FeesCollectorMock::new,
            "fees collector mock",
        );

        b_mock
            .execute_tx(&owner, &sc_wrapper, &rust_zero, |sc| {
                let mut lock_options = MultiValueEncoded::new();
                for option in LOCK_OPTIONS {
                    lock_options.push(*option);
                }

                sc.init(
                    managed_token_id!(BASE_ASSET_TOKEN_ID),
                    MIN_PENALTY_PERCENTAGE,
                    MAX_PENALTY_PERCENTAGE,
                    FEES_BURN_PERCENTAGE,
                    managed_address!(fees_collector_mock.address_ref()),
                    lock_options,
                );

                sc.locked_token()
                    .set_token_id(&managed_token_id!(LOCKED_TOKEN_ID));
            })
            .assert_ok();

        b_mock.set_esdt_local_roles(
            sc_wrapper.address_ref(),
            LOCKED_TOKEN_ID,
            &[
                EsdtLocalRole::NftCreate,
                EsdtLocalRole::NftAddQuantity,
                EsdtLocalRole::NftBurn,
                EsdtLocalRole::Transfer,
            ],
        );

        Self {
            b_mock,
            owner,
            first_user,
            second_user,
            sc_wrapper,
            fees_collector_mock: fees_collector_mock.address_ref().clone(),
        }
    }
}
