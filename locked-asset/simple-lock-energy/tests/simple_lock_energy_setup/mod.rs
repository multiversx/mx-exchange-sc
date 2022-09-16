use elrond_wasm::{
    elrond_codec::multi_types::OptionalValue,
    storage::mappers::StorageTokenWrapper,
    types::{Address, EsdtLocalRole, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper},
    tx_mock::TxResult,
    DebugApi,
};
use elrond_wasm_modules::pause::PauseModule;
use simple_lock::locked_token::LockedTokenModule;
use simple_lock_energy::{lock_options::LockOptionsModule, SimpleLockEnergy};

mod fees_collector_mock;
use fees_collector_mock::*;

pub const EPOCHS_IN_YEAR: u64 = 365;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

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
        let _ = DebugApi::dummy();
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

                assert_eq!(sc.max_lock_option().get(), *LOCK_OPTIONS.last().unwrap());

                sc.locked_token()
                    .set_token_id(&managed_token_id!(LOCKED_TOKEN_ID));
                sc.set_paused(false);
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

        b_mock.set_esdt_balance(
            &first_user,
            BASE_ASSET_TOKEN_ID,
            &rust_biguint!(USER_BALANCE),
        );
        b_mock.set_esdt_balance(
            &second_user,
            BASE_ASSET_TOKEN_ID,
            &rust_biguint!(USER_BALANCE),
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

impl<ScBuilder> SimpleLockEnergySetup<ScBuilder>
where
    ScBuilder: 'static + Copy + Fn() -> simple_lock_energy::ContractObj<DebugApi>,
{
    pub fn lock(
        &mut self,
        caller: &Address,
        token_id: &[u8],
        amount: u64,
        lock_epochs: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.sc_wrapper,
            token_id,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.lock_tokens_endpoint(lock_epochs, OptionalValue::Some(managed_address!(caller)));
            },
        )
    }
}
