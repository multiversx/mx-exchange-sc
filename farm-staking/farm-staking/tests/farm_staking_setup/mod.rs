#![allow(deprecated)]

use external_interaction::ExternalInteractionsModule;
use farm_staking::claim_only_boosted_staking_rewards::ClaimOnlyBoostedStakingRewardsModule;
use farm_staking::compound_stake_farm_rewards::CompoundStakeFarmRewardsModule;
use multiversx_sc::codec::multi_types::OptionalValue;
use multiversx_sc::storage::mappers::StorageTokenWrapper;
use multiversx_sc::types::{Address, BigInt, EsdtLocalRole, ManagedAddress, MultiValueEncoded};
use multiversx_sc_scenario::whitebox_legacy::TxTokenTransfer;
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint, whitebox_legacy::*, DebugApi,
};

pub type RustBigUint = num_bigint::BigUint;

use config::*;
use energy_factory::energy::EnergyModule;
use energy_query::{Energy, EnergyQueryModule};
use farm_boosted_yields::boosted_yields_factors::BoostedYieldsFactorsModule;
use farm_staking::claim_stake_farm_rewards::ClaimStakeFarmRewardsModule;
use farm_staking::custom_rewards::CustomRewardsModule;
use farm_staking::stake_farm::StakeFarmModule;
use farm_staking::token_attributes::{StakingFarmTokenAttributes, UnbondSftAttributes};
use farm_staking::unbond_farm::UnbondFarmModule;
use farm_staking::unstake_farm::UnstakeFarmModule;
use farm_staking::*;
use farm_token::FarmTokenModule;
use pausable::{PausableModule, State};
use permissions_hub::PermissionsHub;
use permissions_hub_module::PermissionsHubModule;
use rewards::RewardsModule;

pub static REWARD_TOKEN_ID: &[u8] = b"RIDE-abcdef"; // reward token ID
pub static FARMING_TOKEN_ID: &[u8] = b"RIDE-abcdef"; // farming token ID
pub static FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
pub const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
pub const MIN_UNBOND_EPOCHS: u64 = 5;
pub const MAX_APR: u64 = 2_500; // 25%
pub const PER_BLOCK_REWARD_AMOUNT: u64 = 5_000;
pub const TOTAL_REWARDS_AMOUNT: u64 = 1_000_000_000_000;

pub const USER_TOTAL_RIDE_TOKENS: u64 = 5_000_000_000;

pub const MAX_PERCENTAGE: u64 = 10_000; // 100%
pub const BOOSTED_YIELDS_PERCENTAGE: u64 = 2_500; // 25%
pub const MAX_REWARDS_FACTOR: u64 = 10;
pub const USER_REWARDS_ENERGY_CONST: u64 = 3;
pub const USER_REWARDS_FARM_CONST: u64 = 2;
pub const MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS: u64 = 1;
pub const WITHDRAW_AMOUNT_TOO_HIGH: &str =
    "Withdraw amount is higher than the remaining uncollected rewards!";

pub struct NonceAmountPair {
    pub nonce: u64,
    pub amount: u64,
}

pub struct FarmStakingSetup<FarmObjBuilder, EnergyFactoryBuilder, PermissionsHubObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    PermissionsHubObjBuilder: 'static + Copy + Fn() -> permissions_hub::ContractObj<DebugApi>,
{
    pub b_mock: BlockchainStateWrapper,
    pub owner_address: Address,
    pub user_address: Address,
    pub user_address2: Address,
    pub farm_wrapper: ContractObjWrapper<farm_staking::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub permissions_hub_wrapper:
        ContractObjWrapper<permissions_hub::ContractObj<DebugApi>, PermissionsHubObjBuilder>,
}

impl<FarmObjBuilder, EnergyFactoryBuilder, PermissionsHubObjBuilder>
    FarmStakingSetup<FarmObjBuilder, EnergyFactoryBuilder, PermissionsHubObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    PermissionsHubObjBuilder: 'static + Copy + Fn() -> permissions_hub::ContractObj<DebugApi>,
{
    pub fn new(
        farm_builder: FarmObjBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        permissions_hub_builder: PermissionsHubObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let farm_wrapper =
            b_mock.create_sc_account(&rust_zero, Some(&owner_addr), farm_builder, "farm-staking");

        let energy_factory_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            energy_factory_builder,
            "energy_factory.wasm",
        );

        let permissions_hub_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            permissions_hub_builder,
            "permissions_hub.wasm",
        );

        b_mock
            .execute_tx(&owner_addr, &permissions_hub_wrapper, &rust_zero, |sc| {
                sc.init();
            })
            .assert_ok();

        // init farm contract

        b_mock
            .execute_tx(&owner_addr, &farm_wrapper, &rust_zero, |sc| {
                let farming_token_id = managed_token_id!(FARMING_TOKEN_ID);
                let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);

                sc.init(
                    farming_token_id,
                    division_safety_constant,
                    managed_biguint!(MAX_APR),
                    MIN_UNBOND_EPOCHS,
                    ManagedAddress::<DebugApi>::zero(),
                    MultiValueEncoded::new(),
                );

                let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
                sc.farm_token().set_token_id(farm_token_id);

                sc.per_block_reward_amount()
                    .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));

                sc.state().set(State::Active);
                sc.produce_rewards_enabled().set(true);

                sc.energy_factory_address()
                    .set(managed_address!(energy_factory_wrapper.address_ref()));

                sc.set_permissions_hub_address(managed_address!(
                    permissions_hub_wrapper.address_ref()
                ));
            })
            .assert_ok();

        b_mock.set_esdt_balance(&owner_addr, REWARD_TOKEN_ID, &TOTAL_REWARDS_AMOUNT.into());
        b_mock
            .execute_esdt_transfer(
                &owner_addr,
                &farm_wrapper,
                REWARD_TOKEN_ID,
                0,
                &TOTAL_REWARDS_AMOUNT.into(),
                |sc| {
                    sc.top_up_rewards();
                },
            )
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

        let user_addr = b_mock.create_user_account(&rust_biguint!(100_000_000));
        b_mock.set_esdt_balance(
            &user_addr,
            FARMING_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
        );
        let user_addr2 = b_mock.create_user_account(&rust_biguint!(100_000_000));
        b_mock.set_esdt_balance(
            &user_addr2,
            FARMING_TOKEN_ID,
            &rust_biguint!(USER_TOTAL_RIDE_TOKENS),
        );

        FarmStakingSetup {
            b_mock,
            owner_address: owner_addr,
            user_address: user_addr,
            user_address2: user_addr2,
            farm_wrapper,
            energy_factory_wrapper,
            permissions_hub_wrapper,
        }
    }

    pub fn stake_farm(
        &mut self,
        user: &Address,
        farm_in_amount: u64,
        additional_farm_tokens: &[TxTokenTransfer],
        expected_farm_token_nonce: u64,
        expected_reward_per_share: u64,
        expected_compounded_reward: u64,
    ) {
        let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
        payments.push(TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(farm_in_amount),
        });
        payments.extend_from_slice(additional_farm_tokens);

        let mut expected_total_out_amount = 0;
        for payment in payments.iter() {
            expected_total_out_amount += payment.value.to_u64_digits()[0];
        }

        self.b_mock
            .execute_esdt_multi_transfer(user, &self.farm_wrapper, &payments, |sc| {
                let (new_farm_token_payment, _) =
                    sc.stake_farm_endpoint(OptionalValue::None).into_tuple();
                assert_eq!(
                    new_farm_token_payment.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(
                    new_farm_token_payment.token_nonce,
                    expected_farm_token_nonce
                );
                assert_eq!(
                    new_farm_token_payment.amount,
                    managed_biguint!(expected_total_out_amount)
                );
            })
            .assert_ok();

        let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            compounded_reward: managed_biguint!(expected_compounded_reward),
            current_farm_amount: managed_biguint!(expected_total_out_amount),
            original_owner: managed_address!(&user),
        };
        self.b_mock.check_nft_balance(
            user,
            FARM_TOKEN_ID,
            expected_farm_token_nonce,
            &rust_biguint!(expected_total_out_amount),
            Some(&expected_attributes),
        );
    }

    pub fn stake_farm_for_other_user(
        &mut self,
        user: &Address,
        broker: &Address,
        farm_in_amount: u64,
        additional_farm_tokens: &[TxTokenTransfer],
    ) -> TxResult {
        let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
        payments.push(TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(farm_in_amount),
        });
        payments.extend_from_slice(additional_farm_tokens);

        self.b_mock
            .execute_esdt_multi_transfer(broker, &self.farm_wrapper, &payments, |sc| {
                sc.stake_farm_endpoint(OptionalValue::Some(managed_address!(user)))
                    .into_tuple();
            })
    }

    pub fn claim_farm_for_other_user(
        &mut self,
        user: &Address,
        broker: &Address,
        farm_in_nonce: u64,
        farm_in_amount: u64,
    ) -> TxResult {
        let mut payments = vec![];
        payments.push(TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: farm_in_nonce,
            value: rust_biguint!(farm_in_amount),
        });

        self.b_mock
            .execute_esdt_multi_transfer(broker, &self.farm_wrapper, &payments, |sc| {
                sc.claim_rewards(OptionalValue::Some(managed_address!(user)))
                    .into_tuple();
            })
    }

    pub fn unstake_farm_for_other_user(
        &mut self,
        user: &Address,
        broker: &Address,
        farm_in_nonce: u64,
        farm_in_amount: u64,
    ) -> TxResult {
        let mut payments = vec![];
        payments.push(TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: farm_in_nonce,
            value: rust_biguint!(farm_in_amount),
        });

        self.b_mock
            .execute_esdt_multi_transfer(broker, &self.farm_wrapper, &payments, |sc| {
                sc.unstake_farm(OptionalValue::Some(managed_address!(user)))
                    .into_tuple();
            })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn claim_rewards(
        &mut self,
        user: &Address,
        farm_token_amount: u64,
        farm_token_nonce: u64,
        expected_reward_token_out: u64,
        expected_user_reward_token_balance: &RustBigUint,
        expected_user_farming_token_balance: &RustBigUint,
        expected_farm_token_nonce_out: u64,
        expected_reward_per_share: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let multi_result = sc.claim_rewards(OptionalValue::None);
                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, expected_farm_token_nonce_out);
                    assert_eq!(first_result.amount, managed_biguint!(farm_token_amount));

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                    assert_eq!(
                        second_result.amount,
                        managed_biguint!(expected_reward_token_out)
                    );
                },
            )
            .assert_ok();

        let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            compounded_reward: managed_biguint!(0),
            current_farm_amount: managed_biguint!(farm_token_amount),
            original_owner: managed_address!(user),
        };

        self.b_mock.check_nft_balance(
            user,
            FARM_TOKEN_ID,
            expected_farm_token_nonce_out,
            &rust_biguint!(farm_token_amount),
            Some(&expected_attributes),
        );
        self.b_mock
            .check_esdt_balance(user, REWARD_TOKEN_ID, expected_user_reward_token_balance);
        self.b_mock
            .check_esdt_balance(user, FARMING_TOKEN_ID, expected_user_farming_token_balance);
    }

    pub fn claim_boosted_rewards_for_user(
        &mut self,
        owner: &Address,
        broker: &Address,
        expected_reward_token_out: u64,
        expected_user_reward_token_balance: &RustBigUint,
    ) {
        self.b_mock
            .execute_tx(broker, &self.farm_wrapper, &rust_biguint!(0u64), |sc| {
                let payment_result =
                    sc.claim_boosted_rewards(OptionalValue::Some(managed_address!(owner)));

                assert_eq!(
                    payment_result.token_identifier,
                    managed_token_id!(REWARD_TOKEN_ID)
                );
                assert_eq!(payment_result.token_nonce, 0);
                assert_eq!(
                    payment_result.amount,
                    managed_biguint!(expected_reward_token_out)
                );
            })
            .assert_ok();

        self.b_mock.check_esdt_balance(
            &self.user_address,
            REWARD_TOKEN_ID,
            expected_user_reward_token_balance,
        );
    }

    pub fn claim_boosted_rewards_for_user_expect_error(
        &mut self,
        owner: &Address,
        broker: &Address,
    ) {
        self.b_mock
            .execute_tx(broker, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                let _ = sc.claim_boosted_rewards(OptionalValue::Some(managed_address!(owner)));
            })
            .assert_error(4, "Cannot claim rewards for this address");
    }

    #[allow(clippy::too_many_arguments)]
    pub fn compound_rewards(
        &mut self,
        user: &Address,
        farm_in_nonce: u64,
        farm_in_amount: u64,
        additional_farm_tokens: &[TxTokenTransfer],
        expected_farm_token_nonce: u64,
        expected_farm_token_amount: u64,
        expected_reward_per_share: u64,
        expected_compounded_reward: u64,
    ) {
        let mut payments = Vec::with_capacity(1 + additional_farm_tokens.len());
        payments.push(TxTokenTransfer {
            token_identifier: FARM_TOKEN_ID.to_vec(),
            nonce: farm_in_nonce,
            value: rust_biguint!(farm_in_amount),
        });
        payments.extend_from_slice(additional_farm_tokens);

        self.b_mock
            .execute_esdt_multi_transfer(user, &self.farm_wrapper, &payments, |sc| {
                let new_farm_token_payment = sc.compound_rewards();
                assert_eq!(
                    new_farm_token_payment.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(
                    new_farm_token_payment.token_nonce,
                    expected_farm_token_nonce
                );
                assert_eq!(
                    new_farm_token_payment.amount,
                    managed_biguint!(expected_farm_token_amount)
                );
            })
            .assert_ok();

        let expected_attributes = StakingFarmTokenAttributes::<DebugApi> {
            reward_per_share: managed_biguint!(expected_reward_per_share),
            compounded_reward: managed_biguint!(expected_compounded_reward),
            current_farm_amount: managed_biguint!(
                expected_farm_token_amount + expected_compounded_reward
            ),
            original_owner: managed_address!(&user),
        };
        self.b_mock.check_nft_balance(
            user,
            FARM_TOKEN_ID,
            expected_farm_token_nonce,
            &rust_biguint!(expected_farm_token_amount + expected_compounded_reward),
            Some(&expected_attributes),
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn unstake_farm(
        &mut self,
        user: &Address,
        farm_token_amount: u64,
        farm_token_nonce: u64,
        expected_rewards_out: u64,
        expected_user_reward_token_balance: &RustBigUint,
        expected_user_farming_token_balance: &RustBigUint,
        expected_new_farm_token_nonce: u64,
        expected_new_farm_token_amount: u64,
        expected_new_farm_token_attributes: &UnbondSftAttributes,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let multi_result = sc.unstake_farm(OptionalValue::None);

                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, expected_new_farm_token_nonce);
                    assert_eq!(
                        first_result.amount,
                        managed_biguint!(expected_new_farm_token_amount)
                    );

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                    assert_eq!(second_result.amount, managed_biguint!(expected_rewards_out));
                },
            )
            .assert_ok();

        self.b_mock.check_nft_balance(
            &self.user_address,
            FARM_TOKEN_ID,
            expected_new_farm_token_nonce,
            &rust_biguint!(expected_new_farm_token_amount),
            Some(expected_new_farm_token_attributes),
        );
        self.b_mock.check_esdt_balance(
            &self.user_address,
            REWARD_TOKEN_ID,
            expected_user_reward_token_balance,
        );
        self.b_mock.check_esdt_balance(
            &self.user_address,
            FARMING_TOKEN_ID,
            expected_user_farming_token_balance,
        );
    }

    pub fn unstake_farm_no_checks(
        &mut self,
        user: &Address,
        farm_token_amount: u64,
        farm_token_nonce: u64,
    ) {
        let _ = self.b_mock.execute_esdt_transfer(
            user,
            &self.farm_wrapper,
            FARM_TOKEN_ID,
            farm_token_nonce,
            &rust_biguint!(farm_token_amount),
            |sc| {
                sc.unstake_farm(OptionalValue::None);
            },
        );
    }

    pub fn unbond_farm(
        &mut self,
        farm_token_nonce: u64,
        farm_tokem_amount: u64,
        expected_farming_token_out: u64,
        expected_user_farming_token_balance: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_address,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_tokem_amount),
                |sc| {
                    let payment = sc.unbond_farm();
                    assert_eq!(
                        payment.token_identifier,
                        managed_token_id!(FARMING_TOKEN_ID)
                    );
                    assert_eq!(payment.token_nonce, 0);
                    assert_eq!(payment.amount, managed_biguint!(expected_farming_token_out));
                },
            )
            .assert_ok();

        self.b_mock.check_esdt_balance(
            &self.user_address,
            FARMING_TOKEN_ID,
            &rust_biguint!(expected_user_farming_token_balance),
        );
    }

    pub fn stake_farm_on_behalf(
        &mut self,
        caller: &Address,
        user: &Address,
        farming_token_amount: u64,
        farm_token_nonce: u64,
        farm_token_amount: u64,
    ) {
        let mut payments = Vec::new();
        payments.push(TxTokenTransfer {
            token_identifier: FARMING_TOKEN_ID.to_vec(),
            nonce: 0,
            value: rust_biguint!(farming_token_amount),
        });

        if farm_token_nonce > 0 {
            payments.push(TxTokenTransfer {
                token_identifier: FARM_TOKEN_ID.to_vec(),
                nonce: farm_token_nonce,
                value: rust_biguint!(farm_token_amount),
            });
        }

        let b_mock = &mut self.b_mock;
        b_mock
            .execute_esdt_multi_transfer(caller, &self.farm_wrapper, &payments, |sc| {
                let stake_farm_result = sc.stake_farm_on_behalf(managed_address!(user));
                let (out_farm_token, _reward_token) = stake_farm_result.into_tuple();
                assert_eq!(
                    out_farm_token.token_identifier,
                    managed_token_id!(FARM_TOKEN_ID)
                );
                assert_eq!(
                    out_farm_token.amount,
                    managed_biguint!(farming_token_amount + farm_token_amount)
                );
            })
            .assert_ok();
    }

    pub fn claim_rewards_on_behalf(
        &mut self,
        caller: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
    ) -> u64 {
        let mut result = 0;
        self.b_mock
            .execute_esdt_transfer(
                caller,
                &self.farm_wrapper,
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (out_farm_token, out_reward_token) =
                        sc.claim_rewards_on_behalf().into_tuple();
                    assert_eq!(
                        out_farm_token.token_identifier,
                        managed_token_id!(FARM_TOKEN_ID)
                    );
                    assert_eq!(out_farm_token.amount, managed_biguint!(farm_token_amount));

                    assert_eq!(
                        out_reward_token.token_identifier,
                        managed_token_id!(REWARD_TOKEN_ID)
                    );

                    result = out_reward_token.amount.to_u64().unwrap();
                },
            )
            .assert_ok();

        result
    }

    pub fn whitelist_address_on_behalf(&mut self, user: &Address, address_to_whitelist: &Address) {
        self.b_mock
            .execute_tx(
                user,
                &self.permissions_hub_wrapper,
                &rust_biguint!(0),
                |sc| {
                    let mut addresses = MultiValueEncoded::new();
                    addresses.push(managed_address!(address_to_whitelist));
                    sc.whitelist(addresses);
                },
            )
            .assert_ok();
    }

    pub fn check_farm_token_supply(&mut self, expected_farm_token_supply: u64) {
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let actual_farm_supply = sc.farm_token_supply().get();
                assert_eq!(
                    managed_biguint!(expected_farm_token_supply),
                    actual_farm_supply
                );
            })
            .assert_ok();
    }

    pub fn check_farm_rps(&mut self, expected_amount: u64) {
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let current_rps = sc.reward_per_share().get();
                assert_eq!(managed_biguint!(expected_amount), current_rps);
            })
            .assert_ok();
    }

    pub fn check_rewards_capacity(&mut self, expected_farm_token_supply: u64) {
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let actual_farm_supply = sc.reward_capacity().get();
                assert_eq!(
                    managed_biguint!(expected_farm_token_supply),
                    actual_farm_supply
                );
            })
            .assert_ok();
    }

    pub fn allow_external_claim_rewards(&mut self, user: &Address, allow_claim: bool) {
        self.b_mock
            .execute_tx(user, &self.farm_wrapper, &rust_biguint!(0), |sc| {
                sc.allow_external_claim(&managed_address!(user))
                    .set(allow_claim);
            })
            .assert_ok();
    }

    pub fn set_block_nonce(&mut self, block_nonce: u64) {
        self.b_mock.set_block_nonce(block_nonce);
    }

    pub fn set_block_epoch(&mut self, block_epoch: u64) {
        self.b_mock.set_block_epoch(block_epoch);
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
                &self.owner_address,
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
            .execute_tx(
                &self.owner_address,
                &self.farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_boosted_yields_rewards_percentage(percentage);
                },
            )
            .assert_ok();
    }

    pub fn set_boosted_yields_factors(&mut self) {
        self.b_mock
            .execute_tx(
                &self.owner_address,
                &self.farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_boosted_yields_factors(
                        managed_biguint!(MAX_REWARDS_FACTOR),
                        managed_biguint!(USER_REWARDS_ENERGY_CONST),
                        managed_biguint!(USER_REWARDS_FARM_CONST),
                        managed_biguint!(MIN_ENERGY_AMOUNT_FOR_BOOSTED_YIELDS),
                        managed_biguint!(MIN_FARM_AMOUNT_FOR_BOOSTED_YIELDS),
                    );
                },
            )
            .assert_ok();
    }

    pub fn withdraw_rewards(&mut self, withdraw_amount: &RustBigUint) {
        self.b_mock
            .execute_tx(
                &self.owner_address,
                &self.farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.withdraw_rewards(withdraw_amount.into());
                },
            )
            .assert_ok();
    }

    pub fn withdraw_rewards_with_error(
        &mut self,
        withdraw_amount: &RustBigUint,
        expected_status: u64,
        expected_message: &str,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner_address,
                &self.farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.withdraw_rewards(withdraw_amount.into());
                },
            )
            .assert_error(expected_status, expected_message)
    }

    pub fn send_position(
        &mut self,
        sender: &Address,
        receiver: &Address,
        nonce: u64,
        amount: u64,
        attr_reward_per_share: u64,
    ) {
        self.b_mock.check_nft_balance(
            sender,
            FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(amount),
            Some(&StakingFarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            }),
        );

        self.b_mock
            .check_nft_balance::<StakingFarmTokenAttributes<DebugApi>>(
                receiver,
                FARM_TOKEN_ID,
                nonce,
                &rust_biguint!(0),
                None,
            );

        self.b_mock.set_nft_balance(
            sender,
            FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(0),
            &StakingFarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            },
        );

        self.b_mock.set_nft_balance(
            receiver,
            FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(amount),
            &StakingFarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            },
        );

        self.b_mock
            .check_nft_balance::<StakingFarmTokenAttributes<DebugApi>>(
                sender,
                FARM_TOKEN_ID,
                nonce,
                &rust_biguint!(0),
                None,
            );

        self.b_mock.check_nft_balance(
            receiver,
            FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(amount),
            Some(&StakingFarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            }),
        );
    }

    pub fn check_user_total_farm_position(&mut self, user_addr: &Address, expected_amount: u64) {
        self.b_mock
            .execute_query(&self.farm_wrapper, |sc| {
                let user_total_farm_position_mapper =
                    sc.user_total_farm_position(&managed_address!(user_addr));
                if expected_amount > 0 && !user_total_farm_position_mapper.is_empty() {
                    assert_eq!(
                        managed_biguint!(expected_amount),
                        user_total_farm_position_mapper.get()
                    );
                }
            })
            .assert_ok();
    }

    pub fn update_energy_for_user(&mut self, user_addr: &Address) {
        self.b_mock
            .execute_tx(
                user_addr,
                &self.energy_factory_wrapper,
                &rust_biguint!(0),
                |sc| {
                    let user_energy =
                        sc.get_updated_energy_entry_for_user(&managed_address!(user_addr));
                    sc.set_energy_entry(&managed_address!(user_addr), user_energy);
                },
            )
            .assert_ok();
    }
}
