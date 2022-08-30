#[cfg(test)]
pub mod migration_tests {
    use elrond_wasm::elrond_codec::Empty;
    use elrond_wasm::storage::mappers::StorageTokenWrapper;
    use elrond_wasm::types::{Address, EsdtLocalRole, MultiValueEncoded};
    use elrond_wasm::elrond_codec::multi_types::OptionalValue;
    use elrond_wasm_debug::tx_mock::TxContextStack;
    use elrond_wasm_debug::{
        managed_address, managed_biguint, managed_token_id, rust_biguint, testing_framework::*,
        DebugApi,
    };

    use common_structs::*;
    use farm::*;
    use farm_v1_2_mock::*;
    use proxy_dex::migration_from_v1_2::*;
    use proxy_dex::proxy_common::*;
    use proxy_dex::proxy_farm::*;
    use proxy_dex::*;

    const PROXY_WASM_PATH: &str = "locked-asset/proxy_dex/output/proxy_dex.wasm";
    const FARM_WASM_PATH: &str = "dex/farm/output/farm.wasm";
    const FARM_V1_2_WASM_PATH: &str = "dex/farm_v1_2_mock/output/farm_v1_2_mock.wasm";
    const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef";
    const LKMEX_TOKEN_ID: &[u8] = b"LKMEX-abcdef";
    const LP_TOKEN_ID: &[u8] = b"LPTOKEN-abcdef";
    const WRAPPED_LP_TOKEN_ID: &[u8] = b"LPTOKEN-abcdef";
    const WRAPPED_FARM_TOKEN_ID: &[u8] = b"WFARM-abcdef";
    const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
    const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000u64;

    #[allow(dead_code)]
    struct ProxySetup<ProxyObjBuilder, FarmObjBuilder, FarmV12MockObjBuilder>
    where
        ProxyObjBuilder: 'static + Copy + Fn() -> proxy_dex::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
        FarmV12MockObjBuilder: 'static + Copy + Fn() -> farm_v1_2_mock::ContractObj<DebugApi>,
    {
        pub blockchain_wrapper: BlockchainStateWrapper,
        pub owner_address: Address,
        pub farm_wrapper: ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>,
        pub farm_v1_2_wrapper:
            ContractObjWrapper<farm_v1_2_mock::ContractObj<DebugApi>, FarmV12MockObjBuilder>,
        pub proxy_wrapper: ContractObjWrapper<proxy_dex::ContractObj<DebugApi>, ProxyObjBuilder>,
    }

    fn setup_proxy<ProxyObjBuilder, FarmObjBuilder, FarmV12MockObjBuilder>(
        proxy_builder: ProxyObjBuilder,
        farm_builder: FarmObjBuilder,
        farm_v1_2_builder: FarmV12MockObjBuilder,
    ) -> ProxySetup<ProxyObjBuilder, FarmObjBuilder, FarmV12MockObjBuilder>
    where
        ProxyObjBuilder: 'static + Copy + Fn() -> proxy_dex::ContractObj<DebugApi>,
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
        FarmV12MockObjBuilder: 'static + Copy + Fn() -> farm_v1_2_mock::ContractObj<DebugApi>,
    {
        let rust_zero = rust_biguint!(0u64);
        let mut blockchain_wrapper = BlockchainStateWrapper::new();
        let owner_addr = blockchain_wrapper.create_user_account(&rust_zero);
        let proxy_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            proxy_builder,
            PROXY_WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(&owner_addr, &proxy_wrapper, &rust_biguint!(0), |sc| {
                sc.init(
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_token_id!(LKMEX_TOKEN_ID),
                    managed_address!(&Address::zero()),
                );

                sc.wrapped_farm_token()
                    .set_token_id(&managed_token_id!(WRAPPED_FARM_TOKEN_ID));
                sc.wrapped_lp_token()
                    .set_token_id(&managed_token_id!(WRAPPED_LP_TOKEN_ID));
            })
            .assert_ok();

        let farm_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            farm_builder,
            FARM_WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(&owner_addr, &farm_wrapper, &rust_biguint!(0), |sc| {
                sc.init(
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_token_id!(LP_TOKEN_ID),
                    managed_biguint!(DIVISION_SAFETY_CONSTANT),
                    managed_address!(&Address::zero()),
                    OptionalValue::None,
                    MultiValueEncoded::new(),
                );
            })
            .assert_ok();

        let farm_v1_2_wrapper = blockchain_wrapper.create_sc_account(
            &rust_zero,
            Some(&owner_addr),
            farm_v1_2_builder,
            FARM_V1_2_WASM_PATH,
        );

        blockchain_wrapper
            .execute_tx(&owner_addr, &farm_v1_2_wrapper, &rust_biguint!(0), |sc| {
                sc.init(
                    managed_address!(&Address::zero()),
                    managed_token_id!(MEX_TOKEN_ID),
                    managed_token_id!(LP_TOKEN_ID),
                    managed_address!(&Address::zero()),
                    managed_biguint!(DIVISION_SAFETY_CONSTANT),
                    managed_address!(&Address::zero()),
                )
                .unwrap();
            })
            .assert_ok();

        blockchain_wrapper
            .execute_tx(&owner_addr, &proxy_wrapper, &rust_biguint!(0), |sc| {
                sc.add_farm_to_intermediate(managed_address!(farm_v1_2_wrapper.address_ref()));
                sc.add_farm_to_intermediate(managed_address!(farm_wrapper.address_ref()));
            })
            .assert_ok();

        blockchain_wrapper.set_esdt_local_roles(
            proxy_wrapper.address_ref(),
            WRAPPED_FARM_TOKEN_ID,
            &[EsdtLocalRole::NftCreate, EsdtLocalRole::NftBurn],
        );

        ProxySetup {
            blockchain_wrapper,
            owner_address: owner_addr,
            farm_wrapper,
            farm_v1_2_wrapper,
            proxy_wrapper,
        }
    }

    #[test]
    fn test_proxy_setup() {
        let _ = setup_proxy(
            proxy_dex::contract_obj,
            farm::contract_obj,
            farm_v1_2_mock::contract_obj,
        );
    }

    #[test]
    fn test_farming_token() {
        let mut proxy_setup = setup_proxy(
            proxy_dex::contract_obj,
            farm::contract_obj,
            farm_v1_2_mock::contract_obj,
        );

        let _ = DebugApi::dummy();

        let owner_addr = proxy_setup.owner_address.clone();
        proxy_setup.blockchain_wrapper.set_nft_balance(
            proxy_setup.proxy_wrapper.address_ref(),
            FARM_TOKEN_ID,
            1,
            &rust_biguint!(100_000),
            &Empty,
        );
        proxy_setup.blockchain_wrapper.set_nft_balance(
            proxy_setup.proxy_wrapper.address_ref(),
            WRAPPED_LP_TOKEN_ID,
            1,
            &rust_biguint!(100_000),
            &Empty,
        );
        proxy_setup.blockchain_wrapper.set_nft_balance(
            &owner_addr,
            WRAPPED_FARM_TOKEN_ID,
            2,
            &rust_biguint!(100_000),
            &WrappedFarmTokenAttributes::<DebugApi> {
                farm_token_id: managed_token_id!(FARM_TOKEN_ID),
                farm_token_nonce: 1,
                farm_token_amount: managed_biguint!(100_000),
                farming_token_id: managed_token_id!(WRAPPED_LP_TOKEN_ID),
                farming_token_nonce: 1,
                farming_token_amount: managed_biguint!(100_000),
            },
        );

        let farm_address = proxy_setup.farm_v1_2_wrapper.address_ref();
        proxy_setup
            .blockchain_wrapper
            .execute_esdt_transfer(
                &owner_addr,
                &proxy_setup.proxy_wrapper,
                WRAPPED_FARM_TOKEN_ID,
                2,
                &rust_biguint!(50_000),
                |sc| {
                    sc.migrate_v1_2_position(managed_address!(farm_address));
                },
            )
            .assert_ok();

        let new_attrs: WrappedFarmTokenAttributes<DebugApi> = proxy_setup
            .blockchain_wrapper
            .get_nft_attributes(&owner_addr, WRAPPED_FARM_TOKEN_ID, 1)
            .unwrap();

        assert_eq!(new_attrs.farming_token_amount, managed_biguint!(50_000));

        let _ = TxContextStack::static_pop();
    }
}
