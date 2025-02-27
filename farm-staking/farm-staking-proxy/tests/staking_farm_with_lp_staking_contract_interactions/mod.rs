#![allow(deprecated)]

use common_structs::FarmTokenAttributes;
use config::ConfigModule;
use energy_factory::energy::EnergyModule;
use energy_query::Energy;
use farm_with_locked_rewards::Farm;
use multiversx_sc::{
    codec::multi_types::OptionalValue,
    types::{Address, BigInt, MultiValueEncoded},
};
use multiversx_sc_scenario::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::TxResult,
    whitebox_legacy::TxTokenTransfer,
    whitebox_legacy::{BlockchainStateWrapper, ContractObjWrapper},
    DebugApi,
};

use farm_staking::{
    compound_stake_farm_rewards::CompoundStakeFarmRewardsModule, stake_farm::StakeFarmModule,
    token_attributes::UnbondSftAttributes, unbond_farm::UnbondFarmModule,
    unstake_farm::UnstakeFarmModule,
};
use farm_staking_proxy::proxy_actions::claim::ProxyClaimModule;
use farm_staking_proxy::{
    dual_yield_token::DualYieldTokenAttributes,
    proxy_actions::external_interaction::ProxyExternalInteractionsModule,
};

use farm_staking_proxy::proxy_actions::stake::ProxyStakeModule;
use farm_staking_proxy::proxy_actions::unstake::ProxyUnstakeModule;

use permissions_hub::PermissionsHub;
use permissions_hub_module::PermissionsHubModule;
use sc_whitelist_module::SCWhitelistModule;

use crate::{
    constants::*,
    staking_farm_with_lp_external_contracts::{setup_energy_factory, setup_lp_farm, setup_pair},
    staking_farm_with_lp_staking_contract_setup::{
        add_proxy_to_whitelist, setup_proxy, setup_staking_farm,
    },
};

pub struct NonceAmountPair {
    pub nonce: u64,
    pub amount: u64,
}

pub struct FarmStakingSetup<
    PairObjBuilder,
    FarmObjBuilder,
    EnergyFactoryBuilder,
    PermissionsHubObjBuilder,
    StakingContractObjBuilder,
    ProxyContractObjBuilder,
> where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    PermissionsHubObjBuilder: 'static + Copy + Fn() -> permissions_hub::ContractObj<DebugApi>,
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    ProxyContractObjBuilder: 'static + Copy + Fn() -> farm_staking_proxy::ContractObj<DebugApi>,
{
    pub owner_addr: Address,
    pub user_addr: Address,
    pub b_mock: BlockchainStateWrapper,
    pub pair_wrapper: ContractObjWrapper<pair::ContractObj<DebugApi>, PairObjBuilder>,
    pub lp_farm_wrapper:
        ContractObjWrapper<farm_with_locked_rewards::ContractObj<DebugApi>, FarmObjBuilder>,
    pub energy_factory_wrapper:
        ContractObjWrapper<energy_factory::ContractObj<DebugApi>, EnergyFactoryBuilder>,
    pub permissions_hub_wrapper:
        ContractObjWrapper<permissions_hub::ContractObj<DebugApi>, PermissionsHubObjBuilder>,
    pub staking_farm_wrapper:
        ContractObjWrapper<farm_staking::ContractObj<DebugApi>, StakingContractObjBuilder>,
    pub proxy_wrapper:
        ContractObjWrapper<farm_staking_proxy::ContractObj<DebugApi>, ProxyContractObjBuilder>,
}

impl<
        PairObjBuilder,
        FarmObjBuilder,
        EnergyFactoryBuilder,
        PermissionsHubObjBuilder,
        StakingContractObjBuilder,
        ProxyContractObjBuilder,
    >
    FarmStakingSetup<
        PairObjBuilder,
        FarmObjBuilder,
        EnergyFactoryBuilder,
        PermissionsHubObjBuilder,
        StakingContractObjBuilder,
        ProxyContractObjBuilder,
    >
where
    PairObjBuilder: 'static + Copy + Fn() -> pair::ContractObj<DebugApi>,
    FarmObjBuilder: 'static + Copy + Fn() -> farm_with_locked_rewards::ContractObj<DebugApi>,
    EnergyFactoryBuilder: 'static + Copy + Fn() -> energy_factory::ContractObj<DebugApi>,
    PermissionsHubObjBuilder: 'static + Copy + Fn() -> permissions_hub::ContractObj<DebugApi>,
    StakingContractObjBuilder: 'static + Copy + Fn() -> farm_staking::ContractObj<DebugApi>,
    ProxyContractObjBuilder: 'static + Copy + Fn() -> farm_staking_proxy::ContractObj<DebugApi>,
{
    pub fn new(
        pair_builder: PairObjBuilder,
        lp_farm_builder: FarmObjBuilder,
        energy_factory_builder: EnergyFactoryBuilder,
        permissions_hub_builder: PermissionsHubObjBuilder,
        staking_farm_builder: StakingContractObjBuilder,
        proxy_builder: ProxyContractObjBuilder,
    ) -> Self {
        let rust_zero = rust_biguint!(0u64);
        let mut b_mock = BlockchainStateWrapper::new();
        let owner_addr = b_mock.create_user_account(&rust_zero);
        let user_addr = b_mock.create_user_account(&rust_biguint!(100_000_000));

        let energy_factory_wrapper =
            setup_energy_factory(&owner_addr, &mut b_mock, energy_factory_builder);
        let pair_wrapper = setup_pair(&owner_addr, &user_addr, &mut b_mock, pair_builder);
        let lp_farm_wrapper = setup_lp_farm(
            &owner_addr,
            &user_addr,
            energy_factory_wrapper.address_ref(),
            &mut b_mock,
            lp_farm_builder,
            USER_TOTAL_LP_TOKENS,
        );
        let staking_farm_wrapper = setup_staking_farm(
            &owner_addr,
            energy_factory_wrapper.address_ref(),
            &mut b_mock,
            staking_farm_builder,
        );
        let proxy_wrapper = setup_proxy(
            &owner_addr,
            lp_farm_wrapper.address_ref(),
            staking_farm_wrapper.address_ref(),
            pair_wrapper.address_ref(),
            &mut b_mock,
            proxy_builder,
        );

        add_proxy_to_whitelist(
            &owner_addr,
            proxy_wrapper.address_ref(),
            &mut b_mock,
            &staking_farm_wrapper,
        );

        let permissions_hub_wrapper = b_mock.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            permissions_hub_builder,
            "permissions_hub.wasm",
        );

        b_mock
            .execute_tx(&owner_addr, &proxy_wrapper, &rust_zero, |sc| {
                sc.set_permissions_hub_address(managed_address!(
                    permissions_hub_wrapper.address_ref()
                ));
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner_addr, &permissions_hub_wrapper, &rust_zero, |sc| {
                sc.init();
            })
            .assert_ok();

        b_mock
            .execute_tx(&owner_addr, &lp_farm_wrapper, &rust_zero, |sc| {
                sc.add_sc_address_to_whitelist(managed_address!(proxy_wrapper.address_ref()));
            })
            .assert_ok();
        b_mock
            .execute_tx(&owner_addr, &energy_factory_wrapper, &rust_zero, |sc| {
                sc.add_sc_address_to_whitelist(managed_address!(lp_farm_wrapper.address_ref()));
            })
            .assert_ok();

        FarmStakingSetup {
            owner_addr,
            user_addr,
            b_mock,
            pair_wrapper,
            lp_farm_wrapper,
            energy_factory_wrapper,
            permissions_hub_wrapper,
            staking_farm_wrapper,
            proxy_wrapper,
        }
    }

    pub fn stake_farm_lp_proxy(
        &mut self,
        lp_farm_token_nonce: u64,
        lp_farm_token_stake_amount: u64,
        expected_staking_farm_token_nonce: u64,
        expected_staking_token_amount: u64,
    ) -> u64 {
        let mut dual_yield_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.proxy_wrapper,
                LP_FARM_TOKEN_ID,
                lp_farm_token_nonce,
                &rust_biguint!(lp_farm_token_stake_amount),
                |sc| {
                    let dual_yield_tokens =
                        sc.stake_farm_tokens(OptionalValue::None).dual_yield_tokens;
                    dual_yield_nonce = dual_yield_tokens.token_nonce;

                    assert_eq!(
                        dual_yield_tokens.amount,
                        managed_biguint!(expected_staking_token_amount)
                    );
                },
            )
            .assert_ok();

        self.b_mock.execute_in_managed_environment(|| {
            let expected_dual_yield_attributes = DualYieldTokenAttributes::<DebugApi> {
                lp_farm_token_nonce,
                lp_farm_token_amount: managed_biguint!(lp_farm_token_stake_amount),
                staking_farm_token_nonce: expected_staking_farm_token_nonce,
                staking_farm_token_amount: managed_biguint!(expected_staking_token_amount),
            };

            self.b_mock.check_nft_balance(
                &self.user_addr,
                DUAL_YIELD_TOKEN_ID,
                dual_yield_nonce,
                &rust_biguint!(expected_staking_token_amount),
                Some(&expected_dual_yield_attributes),
            );
        });

        dual_yield_nonce
    }

    pub fn stake_farm_lp_proxy_multiple(
        &mut self,
        lp_farm_token_nonce: u64,
        lp_farm_token_stake_amount: u64,
        dual_yield_tokens: Vec<NonceAmountPair>,
    ) -> u64 {
        let mut dual_yield_nonce = 0;

        let mut transfers = Vec::new();
        transfers.push(TxTokenTransfer {
            token_identifier: LP_FARM_TOKEN_ID.to_vec(),
            nonce: lp_farm_token_nonce,
            value: rust_biguint!(lp_farm_token_stake_amount),
        });

        for pair in dual_yield_tokens {
            transfers.push(TxTokenTransfer {
                token_identifier: DUAL_YIELD_TOKEN_ID.to_vec(),
                nonce: pair.nonce,
                value: rust_biguint!(pair.amount),
            })
        }

        self.b_mock
            .execute_esdt_multi_transfer(&self.user_addr, &self.proxy_wrapper, &transfers, |sc| {
                let new_dual_yield_token =
                    sc.stake_farm_tokens(OptionalValue::None).dual_yield_tokens;
                dual_yield_nonce = new_dual_yield_token.token_nonce;
            })
            .assert_ok();

        dual_yield_nonce
    }

    pub fn claim_rewards_proxy(
        &mut self,
        dual_yield_token_nonce: u64,
        dual_yield_token_amount: u64,
        expected_lp_farm_reward_amount: u64,
        expected_staking_farm_reward_amount: u64,
        expected_new_dual_yield_token_amount: u64,
    ) -> u64 {
        let mut dual_yield_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.proxy_wrapper,
                DUAL_YIELD_TOKEN_ID,
                dual_yield_token_nonce,
                &rust_biguint!(dual_yield_token_amount),
                |sc| {
                    let received_tokens = sc.claim_dual_yield_endpoint(OptionalValue::None);
                    let lp_farm_rewards = received_tokens.lp_farm_rewards;
                    let staking_farm_rewards = received_tokens.staking_farm_rewards;
                    let new_dual_yield_tokens = received_tokens.new_dual_yield_tokens;

                    dual_yield_nonce = new_dual_yield_tokens.token_nonce;

                    assert_eq!(lp_farm_rewards.amount, expected_lp_farm_reward_amount);
                    assert_eq!(
                        staking_farm_rewards.amount,
                        expected_staking_farm_reward_amount
                    );
                    assert_eq!(
                        new_dual_yield_tokens.amount,
                        expected_new_dual_yield_token_amount
                    );
                },
            )
            .assert_ok();

        dual_yield_nonce
    }

    #[allow(clippy::too_many_arguments)]
    pub fn unstake_proxy(
        &mut self,
        dual_yield_token_nonce: u64,
        dual_yield_token_amount: u64,
        expected_wegld_amount: u64,
        expected_lp_farm_rewards: u64,
        expected_staking_rewards: u64,
        expected_unbond_token_amount: u64,
        expected_unbond_token_unlock_epoch: u64,
    ) -> u64 {
        let mut unbond_token_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.proxy_wrapper,
                DUAL_YIELD_TOKEN_ID,
                dual_yield_token_nonce,
                &rust_biguint!(dual_yield_token_amount),
                |sc| {
                    let received_tokens = sc.unstake_farm_tokens(
                        managed_biguint!(1),
                        managed_biguint!(1),
                        OptionalValue::None,
                    );

                    let wegld_payment = received_tokens.other_token_payment;
                    let lp_farm_rewards = received_tokens.lp_farm_rewards;
                    let staking_rewards = received_tokens.staking_rewards;
                    let unbond_tokens = received_tokens.unbond_staking_farm_token;

                    assert_eq!(wegld_payment.amount, expected_wegld_amount);
                    assert_eq!(lp_farm_rewards.amount, expected_lp_farm_rewards);
                    assert_eq!(staking_rewards.amount, expected_staking_rewards);
                    assert_eq!(unbond_tokens.amount, expected_unbond_token_amount);

                    unbond_token_nonce = unbond_tokens.token_nonce;
                },
            )
            .assert_ok();

        self.b_mock.execute_in_managed_environment(|| {
            let expected_attributes = UnbondSftAttributes {
                unlock_epoch: expected_unbond_token_unlock_epoch,
            };

            self.b_mock.check_nft_balance(
                &self.user_addr,
                STAKING_FARM_TOKEN_ID,
                unbond_token_nonce,
                &rust_biguint!(expected_unbond_token_amount),
                Some(&expected_attributes),
            );
        });

        unbond_token_nonce
    }

    pub fn unbond_proxy(
        &mut self,
        unbond_token_nonce: u64,
        unbond_token_amount: u64,
        expected_token_out_amount: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.staking_farm_wrapper,
                STAKING_FARM_TOKEN_ID,
                unbond_token_nonce,
                &rust_biguint!(unbond_token_amount),
                |sc| {
                    let received_tokens = sc.unbond_farm();
                    assert_eq!(received_tokens.amount, expected_token_out_amount);
                },
            )
            .assert_ok();
    }

    pub fn stake_farm(
        &mut self,
        ride_token_stake_amount: u64,
        expected_staking_token_amount: u64,
    ) -> u64 {
        let mut staking_farm_token_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.staking_farm_wrapper,
                RIDE_TOKEN_ID,
                0,
                &rust_biguint!(ride_token_stake_amount),
                |sc| {
                    let (staking_farm_tokens, _) =
                        sc.stake_farm_endpoint(OptionalValue::None).into_tuple();
                    staking_farm_token_nonce = staking_farm_tokens.token_nonce;

                    assert_eq!(
                        staking_farm_tokens.amount,
                        managed_biguint!(expected_staking_token_amount)
                    );
                },
            )
            .assert_ok();

        staking_farm_token_nonce
    }

    pub fn staking_farm_compound_rewards(
        &mut self,
        farm_token_nonce: u64,
        farm_token_amount: u64,
        expected_new_farm_token_amount: u64,
    ) -> u64 {
        let mut staking_farm_token_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.staking_farm_wrapper,
                STAKING_FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let staking_farm_tokens = sc.compound_rewards();
                    staking_farm_token_nonce = staking_farm_tokens.token_nonce;

                    assert_eq!(
                        staking_farm_tokens.amount,
                        managed_biguint!(expected_new_farm_token_amount)
                    );
                },
            )
            .assert_ok();

        staking_farm_token_nonce
    }

    pub fn staking_farm_unstake(
        &mut self,
        farm_token_nonce: u64,
        farm_token_amount: u64,
        expected_rewards_amount: u64,
        expected_unbond_token_amount: u64,
    ) -> u64 {
        let mut unbond_token_nonce = 0;

        self.b_mock
            .execute_esdt_transfer(
                &self.user_addr,
                &self.staking_farm_wrapper,
                STAKING_FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (unbond_farm_tokens, reward_tokens) =
                        sc.unstake_farm(OptionalValue::None).into_tuple();
                    unbond_token_nonce = unbond_farm_tokens.token_nonce;

                    assert_eq!(reward_tokens.amount, expected_rewards_amount);
                    assert_eq!(
                        unbond_farm_tokens.amount,
                        managed_biguint!(expected_unbond_token_amount)
                    );
                },
            )
            .assert_ok();

        unbond_token_nonce
    }

    pub fn enter_lp_farm(&mut self, user: &Address, farm_token_amount: u64) -> u64 {
        let mut farm_token_nonce = 0;
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.lp_farm_wrapper,
                LP_TOKEN_ID,
                0,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (new_farm_token, _boosted_rewards_payment) =
                        sc.enter_farm_endpoint(OptionalValue::None).into_tuple();
                    assert_eq!(
                        new_farm_token.token_identifier,
                        managed_token_id!(LP_FARM_TOKEN_ID)
                    );
                    assert_eq!(new_farm_token.amount, farm_token_amount);
                    farm_token_nonce = new_farm_token.token_nonce;
                },
            )
            .assert_ok();

        farm_token_nonce
    }

    pub fn exit_lp_farm(&mut self, user: &Address, farm_token_nonce: u64, farm_token_amount: u64) {
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.lp_farm_wrapper,
                LP_FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (_lp_tokens, _boosted_rewards_payment) =
                        sc.exit_farm_endpoint(OptionalValue::None).into_tuple();
                },
            )
            .assert_ok();
    }

    pub fn claim_lp_farm(
        &mut self,
        user: &Address,
        farm_token_nonce: u64,
        farm_token_amount: u64,
        expected_lp_farm_rewards: u64,
    ) -> u64 {
        let mut new_farm_token_nonce = 0;
        self.b_mock
            .execute_esdt_transfer(
                user,
                &self.lp_farm_wrapper,
                LP_FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_token_amount),
                |sc| {
                    let (output_farm_token, boosted_rewards_payment) =
                        sc.claim_rewards_endpoint(OptionalValue::None).into_tuple();
                    assert_eq!(output_farm_token.amount, farm_token_amount);
                    assert_eq!(boosted_rewards_payment.amount, expected_lp_farm_rewards);
                    new_farm_token_nonce = output_farm_token.token_nonce;
                },
            )
            .assert_ok();

        new_farm_token_nonce
    }

    pub fn stake_farm_for_other_user(
        &mut self,
        caller: &Address,
        original_caller: &Address,
        lp_farm_token_nonce: u64,
        lp_farm_token_stake_amount: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.proxy_wrapper,
            LP_FARM_TOKEN_ID,
            lp_farm_token_nonce,
            &rust_biguint!(lp_farm_token_stake_amount),
            |sc| {
                if caller == original_caller {
                    sc.stake_farm_tokens(OptionalValue::None);
                } else {
                    sc.stake_farm_tokens(OptionalValue::Some(managed_address!(original_caller)));
                }
            },
        )
    }

    pub fn claim_dual_yield_for_other_user(
        &mut self,
        caller: &Address,
        original_caller: &Address,
        dual_yield_token_nonce: u64,
        dual_yield_token_amount: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            dual_yield_token_nonce,
            &rust_biguint!(dual_yield_token_amount),
            |sc| {
                if caller == original_caller {
                    sc.claim_dual_yield_endpoint(OptionalValue::None);
                } else {
                    sc.claim_dual_yield_endpoint(OptionalValue::Some(managed_address!(
                        original_caller
                    )));
                }
            },
        )
    }

    pub fn unstake_dual_yield_for_other_user(
        &mut self,
        caller: &Address,
        original_caller: &Address,
        dual_yield_token_nonce: u64,
        dual_yield_token_amount: u64,
    ) -> TxResult {
        self.b_mock.execute_esdt_transfer(
            caller,
            &self.proxy_wrapper,
            DUAL_YIELD_TOKEN_ID,
            dual_yield_token_nonce,
            &rust_biguint!(dual_yield_token_amount),
            |sc| {
                if caller == original_caller {
                    sc.unstake_farm_tokens(
                        managed_biguint!(1u64),
                        managed_biguint!(1u64),
                        OptionalValue::None,
                    );
                } else {
                    sc.unstake_farm_tokens(
                        managed_biguint!(1u64),
                        managed_biguint!(1u64),
                        OptionalValue::Some(managed_address!(original_caller)),
                    );
                }
            },
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn stake_farm_on_behalf(
        &mut self,
        caller: &Address,
        user: &Address,
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: u64,
        additional_dual_yield_token_nonce: u64,
        additional_dual_yield_token_amount: u64,
        expected_dual_yield_token_nonce: u64,
        expected_dual_yield_token_amount: u64,
    ) {
        let mut payments = Vec::new();
        payments.push(TxTokenTransfer {
            token_identifier: LP_FARM_TOKEN_ID.to_vec(),
            nonce: lp_farm_token_nonce,
            value: rust_biguint!(lp_farm_token_amount),
        });

        if additional_dual_yield_token_nonce > 0 {
            payments.push(TxTokenTransfer {
                token_identifier: DUAL_YIELD_TOKEN_ID.to_vec(),
                nonce: additional_dual_yield_token_nonce,
                value: rust_biguint!(additional_dual_yield_token_amount),
            });
        }

        let b_mock = &mut self.b_mock;
        b_mock
            .execute_esdt_multi_transfer(caller, &self.proxy_wrapper, &payments, |sc| {
                let stake_farm_result = sc.stake_farm_on_behalf(managed_address!(user));
                assert_eq!(
                    stake_farm_result.dual_yield_tokens.token_nonce,
                    expected_dual_yield_token_nonce
                );
                assert_eq!(
                    stake_farm_result.dual_yield_tokens.amount,
                    managed_biguint!(expected_dual_yield_token_amount)
                );
            })
            .assert_ok();
    }

    pub fn claim_rewards_on_behalf(
        &mut self,
        caller: &Address,
        dual_yield_token_nonce: u64,
        dual_yield_token_amount: u64,
    ) {
        self.b_mock
            .execute_esdt_transfer(
                caller,
                &self.proxy_wrapper,
                DUAL_YIELD_TOKEN_ID,
                dual_yield_token_nonce,
                &rust_biguint!(dual_yield_token_amount),
                |sc| {
                    let _claim_dual_yield_result = sc.claim_dual_yield_on_behalf();
                },
            )
            .assert_ok();
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

    pub fn set_user_energy(
        &mut self,
        user: &Address,
        energy: u64,
        last_update_epoch: u64,
        locked_tokens: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
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

    pub fn set_lp_farm_boosted_yields_rewards_percentage(
        &mut self,
        boosted_yields_rewards_percentage: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.lp_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.set_boosted_yields_rewards_percentage(boosted_yields_rewards_percentage);
                },
            )
            .assert_ok();
    }

    pub fn set_staking_farm_migration_nonce(&mut self, migration_nonce: u64) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.staking_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.farm_position_migration_nonce().set(migration_nonce);
                },
            )
            .assert_ok();
    }

    pub fn set_lp_farm_migration_nonce(&mut self, migration_nonce: u64) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.lp_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.farm_position_migration_nonce().set(migration_nonce);
                },
            )
            .assert_ok();
    }

    pub fn set_user_total_staking_farm_position(
        &mut self,
        user_addr: &Address,
        new_farm_position: u64,
    ) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.staking_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_total_farm_position(&managed_address!(user_addr))
                        .set(managed_biguint!(new_farm_position));
                },
            )
            .assert_ok();
    }

    pub fn set_user_total_lp_farm_position(&mut self, user_addr: &Address, new_farm_position: u64) {
        self.b_mock
            .execute_tx(
                &self.owner_addr,
                &self.lp_farm_wrapper,
                &rust_biguint!(0),
                |sc| {
                    sc.user_total_farm_position(&managed_address!(user_addr))
                        .set(managed_biguint!(new_farm_position));
                },
            )
            .assert_ok();
    }

    pub fn send_farm_position(
        &mut self,
        sender: &Address,
        receiver: &Address,
        nonce: u64,
        amount: u64,
        attr_reward_per_share: u64,
        attr_entering_epoch: u64,
    ) {
        self.b_mock.check_nft_balance(
            sender,
            LP_FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(amount),
            Some(&FarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                entering_epoch: attr_entering_epoch,
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            }),
        );

        self.b_mock
            .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
                receiver,
                LP_FARM_TOKEN_ID,
                nonce,
                &rust_biguint!(0),
                None,
            );

        self.b_mock.set_nft_balance(
            sender,
            LP_FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(0),
            &FarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                entering_epoch: attr_entering_epoch,
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            },
        );

        self.b_mock.set_nft_balance(
            receiver,
            LP_FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(amount),
            &FarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                entering_epoch: attr_entering_epoch,
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            },
        );

        self.b_mock
            .check_nft_balance::<FarmTokenAttributes<DebugApi>>(
                sender,
                LP_FARM_TOKEN_ID,
                nonce,
                &rust_biguint!(0),
                None,
            );

        self.b_mock.check_nft_balance(
            receiver,
            LP_FARM_TOKEN_ID,
            nonce,
            &rust_biguint!(amount),
            Some(&FarmTokenAttributes::<DebugApi> {
                reward_per_share: managed_biguint!(attr_reward_per_share),
                entering_epoch: attr_entering_epoch,
                compounded_reward: managed_biguint!(0),
                current_farm_amount: managed_biguint!(amount),
                original_owner: managed_address!(&sender),
            }),
        );
    }

    pub fn check_user_total_staking_farm_position(
        &mut self,
        user_addr: &Address,
        expected_amount: u64,
    ) {
        self.b_mock
            .execute_query(&self.staking_farm_wrapper, |sc| {
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

    pub fn check_user_total_lp_farm_position(&mut self, user_addr: &Address, expected_amount: u64) {
        self.b_mock
            .execute_query(&self.lp_farm_wrapper, |sc| {
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
}
