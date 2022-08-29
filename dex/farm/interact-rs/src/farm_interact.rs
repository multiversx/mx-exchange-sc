use std::{
    convert::{TryFrom, TryInto},
    env::Args,
};

use elrond_interact_snippets::{
    elrond_wasm::{
        elrond_codec::{multi_types::MultiValueVec, TopDecode},
        storage::mappers::SingleValue,
        types::{Address, CodeMetadata, ManagedAddress, MultiValueEncoded},
    },
    elrond_wasm_debug::{
        bech32, mandos::interpret_trait::InterpreterContext, mandos_system::model::*, ContractInfo,
        DebugApi,
    },
    env_logger,
    erdrs::interactors::wallet::Wallet,
    tokio, Interactor,
};
use farm::ProxyTrait as _;
use farm_token::ProxyTrait as _;
use load_save::ContractAddressesRaw;

mod load_save;

type FarmContract = ContractInfo<farm::Proxy<DebugApi>>;
type EnergyFactoryContract = ContractInfo<energy_factory_mock::Proxy<DebugApi>>;

static ADDRESS_FILE_PATH: &str = "SavedAddresses.txt";

static GATEWAY: &str = elrond_interact_snippets::erdrs::blockchain::rpc::DEVNET_GATEWAY;
static PEM: &str = "devnetWalletKey.pem";
static SYSTEM_SC_BECH32: &str = "erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u";
const ISSUE_COST: u64 = 50_000_000_000_000_000; // 0.5 EGLD

/// Update these values as needed
static FARM_TOKEN_ID: &[u8] = b"BESTFARM-3b2436";
static FARMING_TOKEN_ID: &[u8] = b"BEST-d3b26b";
static REWARD_TOKEN_ID: &[u8] = b"BEST-d3b26b";
const DIV_SAFETY: u64 = 1_000_000_000_000_000_000;

#[tokio::main]
async fn main() {
    env_logger::init();
    let _ = DebugApi::dummy();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let mut state = State::init().await;
    match cmd.as_str() {
        "deploy" => state.deploy().await,
        "issue_farm_token" => {
            let token_display_name = decode_next_arg(&mut args);
            let token_ticker = decode_next_arg(&mut args);
            let num_decimals = decode_numerical_arg(&mut args);
            state
                .issue_farm_token(token_display_name, token_ticker, num_decimals)
                .await
        }
        _ => panic!("unknown command: {}", &cmd),
    }
}

struct State {
    interactor: Interactor,
    wallet_address: Address,
    farm: FarmContract,
    energy_factory: EnergyFactoryContract,
    raw_addr_expr: ContractAddressesRaw,
}

impl State {
    async fn init() -> Self {
        let mut interactor = Interactor::new(GATEWAY).await;
        let wallet_address = interactor.register_wallet(Wallet::from_pem_file(PEM).unwrap());

        let raw_addr_expr = ContractAddressesRaw::new_from_file(ADDRESS_FILE_PATH.to_string());
        let farm = FarmContract::new(raw_addr_expr.farm_address_expr.clone());
        let energy_factory =
            EnergyFactoryContract::new(raw_addr_expr.energy_factory_address_expr.clone());

        State {
            interactor,
            wallet_address,
            farm,
            energy_factory,
            raw_addr_expr,
        }
    }

    async fn deploy(mut self) {
        let deploy_result: elrond_interact_snippets::InteractorResult<()> = self
            .interactor
            .sc_deploy(
                self.farm
                    .init(
                        REWARD_TOKEN_ID,
                        FARMING_TOKEN_ID,
                        DIV_SAFETY,
                        Address::zero(),
                        MultiValueEncoded::<DebugApi, ManagedAddress<DebugApi>>::new(),
                    )
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .code_metadata(CodeMetadata::all())
                    .contract_code("file:../output/farm.wasm", &InterpreterContext::default())
                    .gas_limit("200,000,000")
                    .expect(TxExpect::ok()),
            )
            .await;

        let new_address = deploy_result.new_deployed_address();
        let new_address_bech32 = bech32::encode(&new_address);
        println!("new address: {}", new_address_bech32);

        self.raw_addr_expr.farm_address_expr = format!("bech32:{}", new_address_bech32);
        self.raw_addr_expr
            .save_to_file(ADDRESS_FILE_PATH.to_string());
    }

    async fn issue_farm_token(
        mut self,
        token_display_name: Vec<u8>,
        token_ticker: Vec<u8>,
        num_decimals: usize,
    ) {
        self.interactor
            .sc_call(
                self.farm
                    .register_farm_token(token_display_name, token_ticker, num_decimals)
                    .into_blockchain_call()
                    .from(&self.wallet_address)
                    .to(self.farm)
                    .egld_value(ISSUE_COST)
                    .gas_limit("200,000,000")
                    .expect(TxExpect::ok()),
            )
            .await;
    }
}

fn decode_next_arg<T: TopDecode>(args: &mut Args) -> T {
    let raw_arg = args.next().unwrap();
    T::top_decode(raw_arg.as_bytes()).unwrap()
}

fn decode_numerical_arg<T: TryFrom<u64>>(args: &mut Args) -> T {
    let raw_arg = args.next().unwrap();
    let full_nr = raw_arg.parse::<u64>().unwrap();
    T::try_from(full_nr).unwrap_or_else(|_| panic!("Could not convert to number"))
}
