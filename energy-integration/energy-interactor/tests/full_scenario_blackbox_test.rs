use energy_interactor_proxies::fees_collector_proxy;
use multiversx_sc_scenario::imports::*;

use energy_factory::*;
use multisig::*;

pub const INIT_EPOCH: u64 = 5;
pub const EPOCHS_IN_YEAR: u64 = 360;
pub const USER_BALANCE: u64 = 1_000_000_000_000_000_000;

pub static LOCK_OPTIONS: &[u64] = &[EPOCHS_IN_YEAR, 2 * EPOCHS_IN_YEAR, 4 * EPOCHS_IN_YEAR];
pub static FIRST_TOKEN_ID: TestTokenIdentifier = TestTokenIdentifier::new("FIRST-123456");
pub static SECOND_TOKEN_ID: TestTokenIdentifier = TestTokenIdentifier::new("SECOND-123456");
pub static BASE_ASSET_TOKEN_ID: TestTokenIdentifier = TestTokenIdentifier::new("MEX-123456");
pub static LOCKED_TOKEN_ID: TestTokenIdentifier = TestTokenIdentifier::new("LOCKED-123456");
pub static LEGACY_LOCKED_TOKEN_ID: TestTokenIdentifier = TestTokenIdentifier::new("LEGACY-123456");
pub static PENALTY_PERCENTAGES: &[u64] = &[4_000, 6_000, 8_000];

const OWNER_ADDRESS: TestAddress = TestAddress::new("owner");
const FIRST_USER: TestAddress = TestAddress::new("first-user");
const SECOND_USER: TestAddress = TestAddress::new("second-user");
const DEPOSITOR: TestAddress = TestAddress::new("depositor");

const FEES_COLLECTOR: TestSCAddress = TestSCAddress::new("fees-collector");
const MULTISIG: TestSCAddress = TestSCAddress::new("multisig");
const ENERGY_FACTORY: TestSCAddress = TestSCAddress::new("energy-factory");

const ENERGY_FACTORY_CODE_PATH: MxscPath = MxscPath::new("tests/energy-factory.mxsc.json");
const MULTISIG_CODE_PATH: MxscPath = MxscPath::new("tests/multisig.mxsc.json");
const FEES_COLLECTOR_CODE_PATH: MxscPath = MxscPath::new("tests/fees-collector.mxsc.json");

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    blockchain.register_contract(ENERGY_FACTORY_CODE_PATH, energy_factory::ContractBuilder);
    blockchain.register_contract(MULTISIG_CODE_PATH, multisig::ContractBuilder);
    blockchain.register_contract(FEES_COLLECTOR_CODE_PATH, fees_collector::ContractBuilder);

    blockchain
}

#[test]
fn full_scenario_blackbox() {
    let mut world = world();

    world.start_trace();

    // account setup
    world.account(OWNER_ADDRESS);
    world.account(FIRST_USER);
    world.account(SECOND_USER);
    world
        .account(DEPOSITOR)
        .balance(50)
        .esdt_balance(FIRST_TOKEN_ID, USER_BALANCE * 2)
        .esdt_balance(SECOND_TOKEN_ID, USER_BALANCE * 2)
        .esdt_nft_balance(LOCKED_TOKEN_ID, 1, USER_BALANCE * 2, ());

    world.current_block().block_epoch(INIT_EPOCH);

    // multisig
    let mut board = MultiValueEncoded::new();
    board.push(ManagedAddress::from(OWNER_ADDRESS.eval_to_array()));

    world
        .tx()
        .from(OWNER_ADDRESS)
        .typed(multisig_proxy::MultisigProxy)
        .init(1usize, board)
        .code(MULTISIG_CODE_PATH)
        .new_address(MULTISIG)
        .returns(ReturnsNewAddress)
        .run();

    // energy factory
    let mut lock_options = MultiValueEncoded::new();
    for (option, penalty) in LOCK_OPTIONS.iter().zip(PENALTY_PERCENTAGES.iter()) {
        lock_options.push((*option, *penalty).into());
    }

    world
        .tx()
        .from(OWNER_ADDRESS)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .init(
            BASE_ASSET_TOKEN_ID,
            LEGACY_LOCKED_TOKEN_ID,
            ManagedAddress::from(ENERGY_FACTORY.eval_to_array()),
            0u64,
            lock_options,
        )
        .code(ENERGY_FACTORY_CODE_PATH)
        .new_address(ENERGY_FACTORY)
        .returns(ReturnsNewAddress)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .set_locked_token_id(LOCKED_TOKEN_ID)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .unpause_endpoint()
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .add_sc_address_to_whitelist(FEES_COLLECTOR)
        .run();

    // fees collector
    world
        .tx()
        .from(OWNER_ADDRESS)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .init(
            LOCKED_TOKEN_ID,
            ManagedAddress::from(ENERGY_FACTORY.eval_to_array()),
        )
        .code(FEES_COLLECTOR_CODE_PATH)
        .new_address(FEES_COLLECTOR)
        .returns(ReturnsNewAddress)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .insert_known_contract(DEPOSITOR)
        .run();

    let mut tokens = MultiValueEncoded::<StaticApi, TokenIdentifier<StaticApi>>::new();
    tokens.push(TokenIdentifier::from(FIRST_TOKEN_ID));
    tokens.push(TokenIdentifier::from(SECOND_TOKEN_ID));
    tokens.push(TokenIdentifier::from(LOCKED_TOKEN_ID));

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .add_known_tokens(tokens)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .set_energy_factory_address(ENERGY_FACTORY)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .set_locking_sc_address(ENERGY_FACTORY)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .set_lock_epochs(LOCK_OPTIONS[2])
        .run();

    // energy factory roles
    let mut roles = MultiValueEncoded::new();
    roles.push(EsdtLocalRole::Mint);
    roles.push(EsdtLocalRole::Burn);

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .set_self_roles(BASE_ASSET_TOKEN_ID, roles)
        .run();

    let mut roles = MultiValueEncoded::new();
    roles.push(EsdtLocalRole::NftCreate);
    roles.push(EsdtLocalRole::NftAddQuantity);
    roles.push(EsdtLocalRole::NftBurn);
    roles.push(EsdtLocalRole::Transfer);

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .set_self_roles(LOCKED_TOKEN_ID, roles)
        .run();

    // fees collector roles
    let mut roles = MultiValueEncoded::new();
    roles.push(EsdtLocalRole::NftBurn);

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .set_self_roles(LOCKED_TOKEN_ID, roles)
        .run();

    // set energy levels
    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .set_energy_for_user(FIRST_USER, 500, 1_000u64)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .set_energy_for_user(SECOND_USER, 500, 9_000u64)
        .run();

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ENERGY_FACTORY)
        .typed(energy_factory_proxy::SimpleLockEnergyProxy)
        .set_energy_for_user(MULTISIG, 9_000, 9_000u64)
        .run();

    // begin scenario
    
    // world
    //     .tx()
    //     .from(OWNER_ADDRESS)
    //     .to(MULTISIG)
    //     .typed(multisig_proxy::MultisigProxy)
    //     .propose_async_call(
    //         FEES_COLLECTOR,
    //         BigUint::default(),
    //         Option::Some(100_000_000u64),
    //         FunctionCall::new("claimRewards"),
    //     )
    //     .run();

    // world
    //     .tx()
    //     .from(OWNER_ADDRESS)
    //     .to(MULTISIG)
    //     .typed(multisig_proxy::MultisigProxy)
    //     .sign_and_perform(1usize)
    //     .run();

    // let claim_progress = world
    //     .query()
    //     .to(FEES_COLLECTOR)
    //     .typed(fees_collector_proxy::FeesCollectorProxy)
    //     .current_claim_progress(MULTISIG)
    //     .returns(ReturnsResult)
    //     .run();

    world
        .tx()
        .from(FIRST_USER)
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .claim_rewards_endpoint(OptionalValue::<ManagedAddress<StaticApi>>::None)
        .run();

    let claim_progress = world
        .query()
        .to(FEES_COLLECTOR)
        .typed(fees_collector_proxy::FeesCollectorProxy)
        .current_claim_progress(FIRST_USER)
        .returns(ReturnsResult)
        .run();

    println!("{:?}", claim_progress);

    world.write_scenario_trace("trace1.scen.json");
}

// pub fn deposit(
//     world: &mut ScenarioWorld,
//     user: TestAddress,
//     token: TestTokenIdentifier,
//     amount: u64,
// ) {
//     world
//         .tx()
//         .from(user)
//         .to(FEES_COLLECTOR)
//         .typed(fees_collector_proxy::FeesCollectorProxy)
//         .deposit_swap_fees()
//         .single_esdt(&TokenIdentifier::from(token), 0u64, &BigUint::from(amount))
//         .run();
// }
