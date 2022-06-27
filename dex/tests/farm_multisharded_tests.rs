use config::{ConfigModule, State};
use elrond_wasm::{
    storage::mappers::StorageTokenWrapper,
    types::{Address, BigUint, EsdtLocalRole, ManagedAddress, MultiValueEncoded},
};
use elrond_wasm_debug::{
    managed_address, managed_biguint, managed_token_id, rust_biguint,
    testing_framework::{BlockchainStateWrapper, ContractObjWrapper, ScCallMandos, TxExpectMandos},
    tx_mock::TxInputESDT,
    DebugApi,
};
use farm::Farm;
use farm_token::FarmTokenModule;
use multishard::MultishardModule;
use rewards::RewardsModule;

type RustBigUint = num_bigint::BigUint;

const FARM_WASM_PATH: &'static str = "farm/output/farm.wasm";

const MEX_TOKEN_ID: &[u8] = b"MEX-abcdef"; // reward token ID
const LP_TOKEN_ID: &[u8] = b"LPTOK-abcdef"; // farming token ID
const FARM_TOKEN_ID: &[u8] = b"FARM-abcdef";
const DIVISION_SAFETY_CONSTANT: u64 = 1_000_000_000_000;
const MIN_FARMING_EPOCHS: u8 = 2;
const PENALTY_PERCENT: u64 = 10;
const PER_BLOCK_REWARD_AMOUNT: u64 = 3_000;

const FARM_COUNT: usize = 3;

struct Context<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    pub blockchain_wrapper: BlockchainStateWrapper,
    pub owner_address: Address,
    pub farm_wrappers:
        [ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>; FARM_COUNT],
}

fn setup_context<FarmObjBuilder>(farm_builder: FarmObjBuilder) -> Context<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);
    let mut blockchain_wrapper = BlockchainStateWrapper::new();
    let owner_address = blockchain_wrapper.create_user_account(&rust_zero);

    let farm_wrappers =
        [0; FARM_COUNT].map(|_| setup_farm(&mut blockchain_wrapper, farm_builder, &owner_address));

    Context {
        blockchain_wrapper,
        owner_address,
        farm_wrappers,
    }
}

fn setup_farm<FarmObjBuilder>(
    blockchain_wrapper: &mut BlockchainStateWrapper,
    farm_builder: FarmObjBuilder,
    owner_address: &Address,
) -> ContractObjWrapper<farm::ContractObj<DebugApi>, FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    let rust_zero = rust_biguint!(0u64);

    let farm_wrapper = blockchain_wrapper.create_sc_account(
        &rust_zero,
        Some(&owner_address),
        farm_builder,
        FARM_WASM_PATH,
    );

    // init farm contract

    blockchain_wrapper
        .execute_tx(&owner_address, &farm_wrapper, &rust_zero, |sc| {
            let reward_token_id = managed_token_id!(MEX_TOKEN_ID);
            let farming_token_id = managed_token_id!(LP_TOKEN_ID);
            let division_safety_constant = managed_biguint!(DIVISION_SAFETY_CONSTANT);
            let pair_address = managed_address!(&Address::zero());

            sc.init(
                reward_token_id,
                farming_token_id,
                division_safety_constant,
                pair_address,
            );

            let farm_token_id = managed_token_id!(FARM_TOKEN_ID);
            sc.farm_token().set_token_id(&farm_token_id);

            sc.per_block_reward_amount()
                .set(&managed_biguint!(PER_BLOCK_REWARD_AMOUNT));
            sc.minimum_farming_epochs().set(&MIN_FARMING_EPOCHS);
            sc.penalty_percent().set(&PENALTY_PERCENT);

            sc.state().set(&State::Active);
            sc.produce_rewards_enabled().set(&true);
        })
        .assert_ok();

    let farm_token_roles = [
        EsdtLocalRole::NftCreate,
        EsdtLocalRole::NftAddQuantity,
        EsdtLocalRole::NftBurn,
    ];
    blockchain_wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        FARM_TOKEN_ID,
        &farm_token_roles[..],
    );

    let farming_token_roles = [EsdtLocalRole::Burn];
    blockchain_wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        LP_TOKEN_ID,
        &farming_token_roles[..],
    );

    let reward_token_roles = [EsdtLocalRole::Mint];
    blockchain_wrapper.set_esdt_local_roles(
        farm_wrapper.address_ref(),
        MEX_TOKEN_ID,
        &reward_token_roles[..],
    );

    farm_wrapper
}

fn to_managed_biguint(value: RustBigUint) -> BigUint<DebugApi> {
    BigUint::from_bytes_be(&value.to_bytes_be())
}

fn to_rust_biguint(value: BigUint<DebugApi>) -> RustBigUint {
    RustBigUint::from_bytes_be(value.to_bytes_be().as_slice())
}

fn check_biguint_eq(actual: BigUint<DebugApi>, expected: RustBigUint, message: &str) {
    assert_eq!(
        actual.clone(),
        to_managed_biguint(expected.clone()),
        "{} Have {}, expected: {}",
        message,
        to_rust_biguint(actual),
        expected,
    );
}

impl<FarmObjBuilder> Context<FarmObjBuilder>
where
    FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
{
    fn setup_farm_whitelists(&mut self) {
        let all_farm_addresses: Vec<&Address> = self
            .farm_wrappers
            .iter()
            .map(|farm| farm.address_ref())
            .collect();
        let caller = &self.owner_address;
        let egld_payment = rust_biguint!(0);
        for farm in &self.farm_wrappers {
            self.blockchain_wrapper
                .execute_tx(caller, &farm, &egld_payment, |sc| {
                    let mut addresses: MultiValueEncoded<DebugApi, ManagedAddress<DebugApi>> =
                        MultiValueEncoded::new();
                    for address in &all_farm_addresses {
                        addresses.push(ManagedAddress::from(address.clone()));
                    }
                    sc.set_sibling_whitelist(addresses);
                })
                .assert_ok();
        }
    }

    fn synchronize_farms(&mut self) {
        let caller = &self.owner_address;
        let egld_payment = rust_biguint!(0);
        for farm in &self.farm_wrappers {
            self.blockchain_wrapper
                .execute_tx(caller, &farm, &egld_payment, |sc| {
                    sc.synchronize();
                })
                .assert_ok();
        }
    }

    fn new_address_with_lp_tokens(&mut self, amount: u64) -> Address {
        let blockchain_wrapper = &mut self.blockchain_wrapper;
        let address = blockchain_wrapper.create_user_account(&rust_biguint!(0));
        blockchain_wrapper.set_esdt_balance(&address, LP_TOKEN_ID, &rust_biguint!(amount));
        address
    }

    fn enter_farm(&mut self, farm_index: usize, caller: &Address, farm_in_amount: u64) {
        let mut payments = Vec::new();
        let farm_in_amount_biguint = rust_biguint!(farm_in_amount);
        payments.push(TxInputESDT {
            token_identifier: LP_TOKEN_ID.to_vec(),
            nonce: 0,
            value: farm_in_amount_biguint.clone(),
        });

        let mut expected_total_out_amount = RustBigUint::default();
        for payment in payments.iter() {
            expected_total_out_amount += payment.value.clone();
        }

        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_multi_transfer(
                &caller,
                &self.farm_wrappers[farm_index],
                &payments,
                |sc| {
                    let payment = sc.enter_farm();
                    assert_eq!(payment.token_identifier, managed_token_id!(FARM_TOKEN_ID));
                    check_biguint_eq(
                        payment.amount,
                        expected_total_out_amount,
                        "Enter farm, farm token payment mismatch.",
                    );
                },
            )
            .assert_ok();

        let mut sc_call = ScCallMandos::new(
            &caller,
            self.farm_wrappers[farm_index].address_ref(),
            "enterFarm",
        );
        sc_call.add_esdt_transfer(LP_TOKEN_ID, 0, &farm_in_amount_biguint);

        let mut tx_expect = TxExpectMandos::new(0);
        tx_expect.add_out_value(&farm_in_amount_biguint.to_bytes_be());

        b_mock.add_mandos_sc_call(sc_call, Some(tx_expect));
    }

    fn exit_farm(
        &mut self,
        farm_index: usize,
        caller: &Address,
        farm_token_nonce: u64,
        farm_out_amount: u64,
        expected_mex_balance: u64,
    ) where
        FarmObjBuilder: 'static + Copy + Fn() -> farm::ContractObj<DebugApi>,
    {
        let b_mock = &mut self.blockchain_wrapper;
        b_mock
            .execute_esdt_transfer(
                &caller,
                &self.farm_wrappers[farm_index],
                FARM_TOKEN_ID,
                farm_token_nonce,
                &rust_biguint!(farm_out_amount),
                |sc| {
                    let multi_result = sc.exit_farm();

                    let (first_result, second_result) = multi_result.into_tuple();

                    assert_eq!(
                        first_result.token_identifier,
                        managed_token_id!(LP_TOKEN_ID)
                    );
                    assert_eq!(first_result.token_nonce, 0);

                    assert_eq!(
                        second_result.token_identifier,
                        managed_token_id!(MEX_TOKEN_ID)
                    );
                    assert_eq!(second_result.token_nonce, 0);
                },
            )
            .assert_ok();

        b_mock.check_esdt_balance(&caller, MEX_TOKEN_ID, &rust_biguint!(expected_mex_balance));
    }

    fn check_supply(
        &mut self,
        farm_index: usize,
        expected_reward_reserve: u64,
        expected_reward_per_share: u64,
        expected_farm_supply: u64,
        expected_local_farm_supply: u64,
        expected_global_farm_supply: u64,
    ) {
        self.blockchain_wrapper
            .execute_query(&self.farm_wrappers[farm_index], |sc| {
                check_biguint_eq(
                    sc.reward_reserve().get(),
                    rust_biguint!(expected_reward_reserve),
                    "Reward reserve mismatch.",
                );
                check_biguint_eq(
                    sc.reward_per_share().get(),
                    rust_biguint!(expected_reward_per_share),
                    "Reward per share mismatch.",
                );
                check_biguint_eq(
                    sc.farm_token_supply().get(),
                    rust_biguint!(expected_farm_supply),
                    "Farm token supply mismatch.",
                );
                check_biguint_eq(
                    sc.local_farm_token_supply().get(),
                    rust_biguint!(expected_local_farm_supply),
                    "Local farm token supply mismatch.",
                );
                check_biguint_eq(
                    sc.global_farm_token_supply().get(),
                    rust_biguint!(expected_global_farm_supply),
                    "Global farm token supply mismatch.",
                );
            })
            .assert_ok();
    }
}

#[test]
fn test_multisharded_reward_distribution() {
    let ctx = &mut setup_context(farm::contract_obj);
    ctx.setup_farm_whitelists();

    let alice = &ctx.new_address_with_lp_tokens(5_000);
    let bob = &ctx.new_address_with_lp_tokens(5_000);
    let carol = &ctx.new_address_with_lp_tokens(5_000);
    let dan = &ctx.new_address_with_lp_tokens(5_000);
    let eve = &ctx.new_address_with_lp_tokens(5_000);

    ctx.blockchain_wrapper.set_block_nonce(10);
    ctx.synchronize_farms();

    ctx.check_supply(0, 10000, 0, 0, 0, 0);
    ctx.check_supply(1, 10000, 0, 0, 0, 0);
    ctx.check_supply(2, 10000, 0, 0, 0, 0);

    // enter the first farm
    ctx.enter_farm(0, alice, 100);
    ctx.enter_farm(0, bob, 200);

    ctx.check_supply(0, 10000, 0, 300, 0, 0);
    ctx.check_supply(1, 10000, 0, 0, 0, 0);
    ctx.check_supply(2, 10000, 0, 0, 0, 0);

    ctx.blockchain_wrapper.set_block_nonce(20);
    ctx.synchronize_farms();

    // only the reward per share of the first farm increases since it's the only one in which users added supply
    // the rewards are 3000, the supply is 300, which means a reward per share of 100 (=3000 / 300)
    ctx.check_supply(0, 40000, 100_000_000_000_000, 300, 300, 300);
    ctx.check_supply(1, 10000, 0, 0, 0, 300);
    ctx.check_supply(2, 10000, 0, 0, 0, 300);

    // enter the second farm
    ctx.enter_farm(1, carol, 350);
    ctx.enter_farm(1, dan, 400);

    // enter the third farm
    ctx.enter_farm(2, eve, 450);

    ctx.blockchain_wrapper.set_block_nonce(30);
    ctx.synchronize_farms();

    // Percentages of rewards distributed to each farm based on its ratio of local to global supply:
    // - first farm:  20% (= (100+200) / 1500)
    // - second farm: 50% (= (350+400) / 1500)
    // - third farm:  30% (= 450 / 1500)
    // the reward per share increases equally because each farm gets a reward directly proportional with how much supply it has
    // From block 20 to block 30, the total reward is 30000 (=3000 * (30 - 20))
    // Farm   | Local supply  | Reward increase      | Reward per share increase
    // First  | 300           | 6000 (= 30000 * 0.2)  | 20 (= 6000 / 300)
    // Second | 750           | 15000 (= 30000 * 0.5) | 20 (= 15000 / 750)
    // Third  | 450           | 9000 (= 30000 * 0.3)  | 20 (= 9000 / 450)
    //
    // This matches the reward per share increase of 20 (=30000 / 1500) that would occur in a scenario that would only have a single farm
    ctx.check_supply(0, 46000, 120_000_000_000_000, 300, 300, 1500);
    ctx.check_supply(1, 25000, 20_000_000_000_000, 750, 750, 1500);
    ctx.check_supply(2, 19000, 20_000_000_000_000, 450, 450, 1500);

    // out of the 36000 (= 46000 - 10000) rewards which accumulated during the last 20 blocks, alice should get one third (= 100 / (100 + 200))
    // which is 12000 (= 1/3 * 36000)
    ctx.exit_farm(0, alice, 1, 100, 12000);
    ctx.check_supply(0, 34000, 120_000_000_000_000, 200, 300, 1500);

    // bob gets the remaining two thirds: 24000 (= 2/3 * 36000)
    ctx.exit_farm(0, bob, 2, 200, 24000);
    ctx.check_supply(0, 10000, 120_000_000_000_000, 0, 300, 1500);

    ctx.blockchain_wrapper.set_block_nonce(40);
    ctx.synchronize_farms();

    // The rewards are distributed as follows:
    // - first farm: 0% (= 0 / 1200)
    // - second farm: 62.5% (= 750 / 1200)
    // - third farm: 37.5% (= 450 / 1200)
    // The reward from block 30 to block 40 is 30000 (=3000 * (40 - 30))
    // Farm   | Local supply  | Reward increase         | Reward per share increase
    // First  | 0             | 0 (= 30000 * 0)         | 0
    // Second | 750           | 18750 (= 30000 * 0.625) | 25 (= 18750 / 750)
    // Third  | 450           | 11250 (= 30000 * 0.375) | 25 (= 11250 / 450)
    //
    // This matches the reward per share increase of 25 (=30000 / 1200) that would occur in a scenario that would only have a single farm
    ctx.check_supply(0, 10000, 120_000_000_000_000, 0, 0, 1200);
    ctx.check_supply(1, 43750, 45_000_000_000_000, 750, 750, 1200);
    ctx.check_supply(2, 30250, 45_000_000_000_000, 450, 450, 1200);
}

#[test]
fn test_multisharded_exit_before_sync_should_not_give_rewards() {
    let ctx = &mut setup_context(farm::contract_obj);
    ctx.setup_farm_whitelists();

    let alice = &ctx.new_address_with_lp_tokens(5_000);
    let bob = &ctx.new_address_with_lp_tokens(5_000);

    ctx.blockchain_wrapper.set_block_nonce(10);
    ctx.synchronize_farms();

    ctx.check_supply(0, 10000, 0, 0, 0, 0);
    ctx.check_supply(1, 10000, 0, 0, 0, 0);
    ctx.check_supply(2, 10000, 0, 0, 0, 0);

    ctx.enter_farm(0, alice, 100);

    ctx.blockchain_wrapper.set_block_nonce(20);
    ctx.synchronize_farms();

    ctx.check_supply(0, 40000, 300_000_000_000_000, 100, 100, 100);

    ctx.enter_farm(0, bob, 200);
    ctx.check_supply(0, 40000, 300_000_000_000_000, 300, 100, 100);

    ctx.blockchain_wrapper.set_block_nonce(30);

    // check that bob receives 0 rewards - an incremented nonce has no effect if no sync has been done
    ctx.exit_farm(0, bob, 2, 200, 0);
    ctx.check_supply(0, 40000, 300_000_000_000_000, 100, 100, 100);
}
