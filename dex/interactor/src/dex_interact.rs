mod dex_interact_cli;
mod dex_interact_config;
mod dex_interact_pair;
mod dex_interact_state;
mod pair_proxy;

use clap::Parser;
use dex_interact_config::Config;
use dex_interact_state::State;
use multiversx_sc_snippets::imports::*;

const INTERACTOR_SCENARIO_TRACE_PATH: &str = "interactor_trace.scen.json";

#[tokio::main]
async fn main() {
    env_logger::init();

    let mut dex_interact = DexInteract::init().await;
    dex_interact.register_wallets();

    let cli = dex_interact_cli::InteractCli::parse();
    match &cli.command {
        Some(dex_interact_cli::InteractCliCommand::Pause) => {
            dex_interact.pause().await;
        }
        Some(dex_interact_cli::InteractCliCommand::Swap(args)) => {
            dex_interact
                .swap_tokens_fixed_input(args.amount, &args.token_identifier, args.min_amount)
                .await;
        }
        None => {}
    }
}

struct DexInteract {
    interactor: Interactor,
    wallet_address: Bech32Address,
    state: State,
}

impl DexInteract {
    async fn init() -> Self {
        let config = Config::load_config();
        let mut interactor = Interactor::new(config.gateway())
            .await
            .with_tracer(INTERACTOR_SCENARIO_TRACE_PATH)
            .await;
        let test_address = test_wallets::mike();
        let wallet_address = interactor.register_wallet(test_address);
        println!("wallet address: {:#?}", test_address.address());

        Self {
            interactor,
            wallet_address: wallet_address.into(),
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

    async fn pause(&mut self) {
        println!("Attempting to pause pair contract...");

        self.interactor
            .tx()
            .from(&self.wallet_address)
            .to(self.state.current_pair_address())
            .typed(pair_proxy::PairProxy)
            .pause()
            .prepare_async()
            .run()
            .await;
    }
}
