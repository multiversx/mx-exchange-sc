#![allow(dead_code)]
#![allow(deprecated)]

use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::{
    storage::mappers::StorageTokenWrapper,
    types::{Address, BigInt, EsdtLocalRole, MultiValueEncoded},
};
use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use energy_factory_mock::EnergyFactoryMock;
use energy_query::{Energy, EnergyQueryModule};
use energy_update::EnergyUpdate;
use farm::Farm;
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule;
use farm_boosted_yields::FarmBoostedYieldsModule;
use farm_token::FarmTokenModule;
use pausable::{PausableModule, State};
use sc_whitelist_module::SCWhitelistModule;
use week_timekeeping::Epoch;
use weekly_rewards_splitting::update_claim_progress_energy::UpdateClaimProgressEnergyModule;

pub static REWARD_TOKEN_ID: &[u8] = b"REW-123456";
pub static FARMING_TOKEN_ID: &[u8] = b"LPTOK-123456";
pub static FARM_TOKEN_ID: &[u8] = b"FARM-123456";
pub const DIV_SAFETY: u64 = 1_000_000_000_000;
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 1_000;
pub const FARMING_TOKEN_BALANCE: u64 = 200_000_000;
pub const BOOSTED_YIELDS_PERCENTAGE: u64 = 2_500; // 25%
pub const MAX_REWARDS_FACTOR: u64 = 10;
pub const USER_REWARDS_ENERGY_CONST: u64 = 3;
pub const USER_REWARDS_FARM_CONST: u64 = 2;
pub const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;

pub struct RawFarmTokenAttributes {
    pub reward_per_share_bytes: Vec<u8>,
    pub entering_epoch: Epoch,
    pub compounded_reward_bytes: Vec<u8>,
    pub current_farm_amount_bytes: Vec<u8>,
    pub original_owner_bytes: [u8; 32],
}

pub struct MultiUserFarmSetup<FarmObjBuilder, EnergyFactoryBuilder, EnergyUpdateObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
    EnergyUpdateObjBuilder: 'static + Copy + Fn() -> energy_update::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner: Address,
    pub first_user: Address,
    pub second_user: Address,
    pub third_user: Address,
    pub last_farm_token_nonce: u64,
    pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory_mock::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub eu_wrapper:
        ContractObjWrapper<energy_update::ContractObj<DebugApi>, EnergyUpdateObjBuilder>,
}

impl<FarmObjBuilder, EnergyFactoryBuilder, EnergyUpdateObjBuilder>
    MultiUserFarmSetup<FarmObjBuilder, EnergyFactoryBuilder, EnergyUpdateObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory_mock::ContractObj<DebugApi>,
    EnergyUpdateObjBuilder: 'static + Copy + Fn() -> energy_update::ContractObj<DebugApi>,
{
    pub fn new(
        farm_builder: FarmObjBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        eu_builder: EnergyUpdateObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner = b_mock.create_user_account(&rust_zero);
        let first_user = b_mock.create_user_account(&rust_zero);
        let second_user = b_mock.create_user_account(&rust_zero);
        let third_user = b_mock.create_user_account(&rust_zero);
        let farm_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), farm_builder, "farm.wasm");
        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner),
            energy_factory_builder,
            "energy_factory.wasm",
        );
        let eu_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner), eu_builder, "energy update mock");

        b_mock
            .execute_tx(&owner, &eu_wrapper, &rust_zero, |sc| {
                sc.init();
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner, &farm_wrapper, &rust_zero, |sc| {
                let reward_token_id = managed_token_id!(REWARD_TOKEN_ID);
                let farming_token_id = managed_token_id!(FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIV_SAFETY);
                let pair_address = managed_address!(&Address::zero());

                sc.init(
                    reward_token_id,
                    farming_token_id,
                    division_safety_constant,
                    pair_address,
                    managed_address!(&owner),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

                sc.per_block_reward_amount()
                    .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);
                sc.set_energy_factory_address(managed_address!(
                    energy_factory_wrapper.address_ref()
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
            &first_user,
            FARMING_TOKEN_ID,
            &rust_biguint!(FARMING_TOKEN_BALANCE),
        );
        b_mock.set_esdt_balance(
            &second_user,
            FARMING_TOKEN_ID,
            &rust_biguint!(FARMING_TOKEN_BALANCE),
        );
        b_mock.set_esdt_balance(
            &third_user,
            FARMING_TOKEN_ID,
            &rust_biguint!(FARMING_TOKEN_BALANCE),
        );

        MultiUserFarmSetup {
            b_mock,
            owner,
            first_user,
            second_user,
            third_user,
            last_farm_token_nonce: 0,
            farm_wrapper,
            energy_factory_wrapper,
            eu_wrapper,
        }
    }

    pub fn set_user_energy(
        &mut self,
        user: &Address,
        energy: u64,
        last_update_epoch: u64,
        locked_tokens: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_energy(&managed_address!(user)).set(&Energy::new(
                        BigInt::from(managed_biguint!(energy)),
                        last_update_epoch,
                        managed_biguint!(locked_tokens),
                    ));
                },
            )
            .assert_ok();
    }

    pub fn set_boosted_yields_rewards_percentage(&mut self, percentage: u64) {
        self.b_mock
            .execute_tx(&self.owner, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.set_boosted_yields_rewards_percentage(percentage);
            })
            .assert_ok();
    }

    pub fn set_boosted_yields_factors(&mut self) {
        self.b_mock
            .execute_tx(&self.owner, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.set_boosted_yields_factors(
                    managed_biguint!(MAX_REWARDS_FACTOR),
                    managed_biguint!(USER_REWARDS_ENERGY_CONST),
                    managed_biguint!(USER_REWARDS_FARM_CONST),
                    managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                    managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                );
            })
            .assert_ok();
    }

    pub fn add_known_proxy(&mut self, known_proxy: &Address) {
        self.b_mock
            .execute_tx(&self.owner, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.add_sc_address_to_whitelist(managed_address!(known_proxy));
            })
            .assert_ok();
    }

    pub fn enter_farm(&mut self, user: &Address, farming_token_amount: u64) {
        self.last_farm_token_nonce += 1;

        let expected_farm_token_nonce = self.last_farm_token_nonce;
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARMING_TOKEN_ID,
                0,
                &rust_biguint!(farming_token_amount),
                |sc| {
                    let enter_farm_result = sc.enter_farm_endpoint(OptionalValue::None);
                    let (out_farm_token, _reward_token) = enter_farm_result.into_tuple();
                    assert_eq!(
                        out_farm_token.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                    assert_eq!(
                        out_farm_token.amount,
                        managed_biguint!(farming_token_amount)
                    );
                },
            )
            .assert_ok();
    }

    pub fn enter_farm_with_additional_payment(
        &mut self,
        user: &Address,
        farming_token_amount: u64,
        farm_token_nonce: u64,
        farm_token_amount: u64,
    ) -> u64 {
        self.last_farm_token_nonce += 1;
        let mut result = 0;
        let expected_farm_token_nonce = self.last_farm_token_nonce;

        let mut payments = Vec::new();
        payments.push(TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(farming_token_amount),
        });
        payments.push(TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: farm_token_nonce,
            value: rust_biguint!(farm_token_amount),
        });

        self.b_mock
            .execute_esdt_multi_transfer(user, &self.farm_wrapper, &payments, |sc| {
                let enter_farm_result = sc.enter_farm_endpoint(OptionalValue::None);
                let (out_farm_token, reward_token) = enter_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                assert_eq!(
                    out_farm_token.amount,
                    managed_biguint!(farming_token_amount + farm_token_amount)
                );
                result = reward_token.amount.to_u64().unwrap();
            })
            .assert_ok();

        result
    }

    pub fn merge_farm_tokens(
        &mut self,
        user: &Address,
        first_token_nonce: u64,
        first_token_amount: u64,
        second_token_nonce: u64,
        second_token_amount: u64,
    ) {
        self.last_farm_token_nonce += 1;
        let expected_farm_token_nonce = self.last_farm_token_nonce;
        let expected_farm_token_amount = first_token_amount + second_token_amount;
        let mut payments = Vec::new();
        payments.push(TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: first_token_nonce,
            value: rust_biguint!(first_token_amount),
        });
        payments.push(TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: second_token_nonce,
            value: rust_biguint!(second_token_amount),
        });

        self.b_mock
            .execute_esdt_multi_transfer(user, &self.farm_wrapper, &payments, |sc| {
                let out_farm_token = sc.merge_farm_tokens_endpoint(OptionalValue::None);
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                assert_eq!(
                    out_farm_token.amount,
                    managed_biguint!(expected_farm_token_amount)
                );
            })
            .assert_ok();
    }

    pub fn calculate_rewards(
        &mut self,
        user: &Address,
        farm_token_amount: u64,
        attributes: FarmTokenAttributes<DebugApi>,
    ) -> u64 {
        let mut result = 0;

        let raw_attributes = RawFarmTokenAttributes {
            reward_per_share_bytes: attributes
                .reward_per_share
                .to_bytes_be()
                .as_slice()
                .to_vec(),
            entering_epoch: attributes.entering_epoch,
            compounded_reward_bytes: attributes
                .compounded_reward
                .to_bytes_be()
                .as_slice()
                .to_vec(),
            current_farm_amount_bytes: attributes
                .current_farm_amount
                .to_bytes_be()
                .as_slice()
                .to_vec(),
            original_owner_bytes: attributes.original_owner.to_byte_array(),
        };

        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let attributes_managed = FarmTokenAttributes {
                    reward_per_share: multiversx_sc::types::BigUint::<DebugApi>::from_bytes_be(
                        &raw_attributes.reward_per_share_bytes,
                    ),
                    entering_epoch: raw_attributes.entering_epoch,
                    compounded_reward: multiversx_sc::types::BigUint::<DebugApi>::from_bytes_be(
                        &raw_attributes.compounded_reward_bytes,
                    ),
                    current_farm_amount: multiversx_sc::types::BigUint::<DebugApi>::from_bytes_be(
                        &raw_attributes.current_farm_amount_bytes,
                    ),
                    original_owner:
                        multiversx_sc::types::ManagedAddress::<DebugApi>::new_from_bytes(
                            &raw_attributes.original_owner_bytes,
                        ),
                };

                let result_managed = sc.calculate_rewards_for_given_position(
                    managed_address!(user),
                    managed_biguint!(farm_token_amount),
                    attributes_managed,
                );
                result = result_managed.to_u64().unwrap();
            })
            .assert_ok();

        result
    }

    pub fn claim_rewards(
        &mut self,
        user: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
    ) -> u64 {
        self.last_farm_token_nonce += 1;

        let expected_farm_token_nonce = self.last_farm_token_nonce;
        let mut result = 0;
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (out_farm_token, out_reward_token) =
                        sc.claim_rewards_endpoint(OptionalValue::None).into_tuple();
                    assert_eq!(
                        out_farm_token.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                    assert_eq!(out_farm_token.amount, managed_biguint!(farm_token_amount));

                    assert_eq!(
                        out_reward_token.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );
                    assert_eq!(out_reward_token.token_nonce, 0);

                    result = out_reward_token.amount.to_u64().unwrap();
                },
            )
            .assert_ok();

        result
    }

    pub fn claim_rewards_known_proxy(
        &mut self,
        user: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
        known_proxy: &Address,
    ) -> u64 {
        self.last_farm_token_nonce += 1;

        let expected_farm_token_nonce = self.last_farm_token_nonce;
        let mut result = 0;
        self.b_mock
            .execute_esdt_transfer(
                known_proxy,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (out_farm_token, out_reward_token) = sc
                        .claim_rewards_endpoint(OptionalValue::Some(managed_address!(user)))
                        .into_tuple();
                    assert_eq!(
                        out_farm_token.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(out_farm_token.token_nonce, expected_farm_token_nonce);
                    assert_eq!(out_farm_token.amount, managed_biguint!(farm_token_amount));

                    assert_eq!(
                        out_reward_token.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );
                    assert_eq!(out_reward_token.token_nonce, 0);

                    result = out_reward_token.amount.to_u64().unwrap();
                },
            )
            .assert_ok();

        result
    }

    pub fn exit_farm(
        &mut self,
        user: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
        exit_farm_amount: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let _ = sc.exit_farm_endpoint(
                        managed_biguint!(exit_farm_amount),
                        OptionalValue::None,
                    );
                },
            )
            .assert_ok();
    }

    pub fn exit_farm_known_proxy(
        &mut self,
        user: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
        exit_farm_amount: u64,
        known_proxy: &Address,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                known_proxy,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let _ = sc.exit_farm_endpoint(
                        managed_biguint!(exit_farm_amount),
                        OptionalValue::Some(managed_address!(user)),
                    );
                },
            )
            .assert_ok();
    }

    pub fn update_energy_for_user(&mut self) {
        let b_mock = &mut self.b_mock;
        let user_addr = &self.first_user;
        let _ = b_mock.execute_tx(
            &self.first_user,
            &self.farm_wrapper,
            &rust_biguint!(0),
            |sc| {
                sc.update_energy_for_user(managed_address!(user_addr));
            },
        );
    }

    pub fn check_farm_claim_progress_energy(&mut self, expected_user_energy: u64) {
        let b_mock = &mut self.b_mock;
        let user_addr = &self.first_user;
        b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let current_claim_progress_mapper =
                    sc.current_claim_progress(&managed_address!(user_addr));
                if expected_user_energy > 0 {
                    assert_eq!(
                        managed_biguint!(expected_user_energy),
                        current_claim_progress_mapper
                            .get()
                            .energy
                            .get_energy_amount()
                    );
                } else {
                    assert!(current_claim_progress_mapper.is_empty())
                }
            })
            .assert_ok();
    }

    pub fn check_error_collect_undistributed_boosted_rewards(&mut self, expected_message: &str) {
        self.b_mock
            .execute_tx(&self.owner, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.collect_undistributed_boosted_rewards();
            })
            .assert_error(4, expected_message)
    }

    pub fn collect_undistributed_boosted_rewards(&mut self) {
        self.b_mock
            .execute_tx(&self.owner, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.collect_undistributed_boosted_rewards();
            })
            .assert_ok();
    }

    pub fn check_remaining_boosted_rewards_to_distribute(
        &mut self,
        week: u64,
        expected_amount: u64,
    ) {
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let result_managed = sc
                    .remaining_boosted_rewards_to_distribute(week as usize)
                    .get();
                assert_eq!(result_managed, managed_biguint!(expected_amount));
            })
            .assert_ok();
    }

    pub fn check_undistributed_boosted_rewards(&mut self, expected_amount: u64) {
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let result_managed = sc.undistributed_boosted_rewards().get();
                assert_eq!(result_managed, managed_biguint!(expected_amount));
            })
            .assert_ok();
    }
}
