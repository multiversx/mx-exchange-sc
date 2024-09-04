mod energy_factory;
mod energy_interact_config;
mod energy_interact_state;
mod fees_collector;
mod multisig;
mod structs;

use energy_interact_config::Config;
use energy_interact_state::State;
use multiversx_sc_snippets::imports::*;

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut dex_interact = DexInteract::init().await;
    dex_interact.register_wallets();
}

struct DexInteract {
    interactor: Interactor,
    wallet_address: Bech32Address,
    second_wallet_address: Bech32Address,
    state: State,
}

impl DexInteract {
    async fn init() -> Self {
        let config = Config::load_config();
        let mut interactor = Interactor::new(config.gateway()).await;

        let wallet_address = interactor.register_wallet(test_wallets::mike());
        let second_wallet_address = interactor.register_wallet(test_wallets::alice());

        Self {
            interactor,
            wallet_address: wallet_address.into(),
            second_wallet_address: second_wallet_address.into(),
            state: State::load_state(),
        }
    }

    fn register_wallets(&mut self) {
        let carol = test_wallets::carol();
        let dan = test_wallets::dan();
        let eve = test_wallets::eve();

        for wallet in &[carol, dan, eve] {
            self.interactor.register_wallet(*wallet);
        }
    }
}

#[tokio::test]
async fn debug_test() {
    let mut interact = DexInteract::init().await;

    let to = Bech32Address::from_bech32_string(
        "erd1qqqqqqqqqqqqqpgquq94exc0fs6x8tvzyzsmj2v643vmclct0n4shkztah".to_string(),
    );
    let egld_amount = RustBigUint::ZERO;
    let opt_gas_limit = Option::Some(100_000_000u64);
    let function_call = "claimRewards";

    let action_id = multisig::propose_async_call(
        &mut interact,
        to.to_address(),
        egld_amount,
        opt_gas_limit,
        function_call,
    )
    .await;

    let address = multisig::sign_and_perform(&mut interact, action_id).await;
    match address {
        OptionalValue::Some(val) => println!("address is {:?}", val),
        OptionalValue::None => println!("no address"),
    }
}
