use elrond_wasm::types::{Address, BigInt, MultiValueEncoded};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
    tx_mock::TxResult, DebugApi,
};
use energy_factory_mock::EnergyFactoryMock;
use energy_query::{Energy, EnergyQueryModule};
use fees_collector::{config::ConfigModule, fees_accumulation::FeesAccumulationModule, *};
use week_timekeeping::{Week, WeekTimekeepingModule, EPOCHS_IN_WEEK};

const INIT_EPOCH: u64 = 5;

pub static FIRST_TOKEN_ID: &[u8] = b"FIRST-123456";
pub static SECOND_TOKEN_ID: &[u8] = b"SECOND-123456";
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

pub struct FeesCollectorSetup<FeesCollectorObjBuilder, EnergyFactoryObjBuilder>
where
    FeesCollectorObjBuilder: 'static + Copy + Fn() -> fees_collector::ContractObj<DebugApi>,
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub depositor_address: Address,
    pub fc_wrapper:
        ContractObjWrapper<fees_collector::ContractObj<DebugApi>, FeesCollectorObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory_mock::ContractObj<DebugApi>, EnergyFactoryObjBuilder>,
    pub current_epoch: u64,
}

impl<FeesCollectorObjBuilder, EnergyFactoryObjBuilder>
    FeesCollectorSetup<FeesCollectorObjBuilder, EnergyFactoryObjBuilder>
where
    FeesCollectorObjBuilder: 'static + Copy + Fn() -> fees_collector::ContractObj<DebugApi>,
    EnergyFactoryObjBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
{
    pub fn new(
        fc_builder: FeesCollectorObjBuilder,
        energy_factory_builder: EnergyFactoryObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_address = b_mock.create_user_account(&rust_zero);
        let depositor_address = b_mock.create_user_account(&rust_zero);
        let fc_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_address),
            fc_builder,
            "fees collector path",
        );
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_address),
            energy_factory_builder,
            "energy factory path",
        );

        b_mock.set_esdt_balance(
            &depositor_address,
            FIRST_TOKEN_ID,
            &rust_biguint!(USER_BALANCE * 2),
        );
        b_mock.set_esdt_balance(
            &depositor_address,
            SECOND_TOKEN_ID,
            &rust_biguint!(USER_BALANCE * 2),
        );

        b_mock.set_block_epoch(INIT_EPOCH);

        b_mock
            .execute_tx(&owner_address, &fc_wrapper, &rust_zero, |sc| {
                sc.init(
                    managed_token_id!(FIRST_TOKEN_ID),
                    managed_address!(energy_factory_wrapper.address_ref()),
                );

                let _ = sc
                    .known_contracts()
                    .insert(managed_address!(&depositor_address));

                let mut tokens = MultiValueEncoded::new();
                tokens.push(managed_token_id!(FIRST_TOKEN_ID));
                tokens.push(managed_token_id!(SECOND_TOKEN_ID));

                sc.add_known_tokens(tokens);

                sc.set_energy_factory_address(managed_address!(
                    energy_factory_wrapper.address_ref()
                ));
            })
            .assert_ok();

        FeesCollectorSetup {
            b_mock,
            owner_address,
            depositor_address,
            fc_wrapper,
            energy_factory_wrapper,
            current_epoch: INIT_EPOCH,
        }
    }

    pub fn advance_week(&mut self) {
        self.current_epoch += EPOCHS_IN_WEEK;
        self.b_mock.set_block_epoch(self.current_epoch);
    }

    pub fn get_current_week(&mut self) -> Week {
        let mut result = 0;
        self.b_mock
            .execute_query(&self.fc_wrapper, |sc| result = sc.get_current_week())
            .assert_ok();

        result
    }

    pub fn deposit(&mut self, token: &[u8], amount: u64) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            &self.depositor_address,
            &self.fc_wrapper,
            token,
            0,
            &rust_biguint!(amount),
            |sc| {
                sc.deposit_swap_fees();
            },
        )
    }

    pub fn claim(&mut self, user: &Address) -> TxResult {
        self.b_mock
            .execute_tx(user, &self.fc_wrapper, &rust_biguint!(0), |sc| {
                let _ = sc.claim_rewards();
            })
    }

    pub fn set_energy(&mut self, user: &Address, total_locked_tokens: u64, energy_amount: u64) {
        let current_epoch = self.current_epoch;
        self.b_mock
            .execute_tx(
                user,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(user)).set(&Energy::new(
                        BigInt::from(managed_biguint!(energy_amount)),
                        current_epoch,
                        managed_biguint!(total_locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }
}
