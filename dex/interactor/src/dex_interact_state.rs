use multiversx_sc_scenario::imports::Bech32Address;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

/// State file
const STATE_FILE: &str = "state.toml";

/// Multisig Interact state
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    farm_with_locked_rewards: Option<Bech32Address>,
    pair: Option<Bech32Address>,
    router: Option<Bech32Address>,
    farm_staking: Option<Bech32Address>,
    farm_staking_proxy: Option<Bech32Address>,
    energy_factory: Option<Bech32Address>,
    first_token_id: Option<String>,
    second_token_id: Option<String>,
}

impl State {
    // Deserializes state from file
    pub fn load_state() -> Self {
        if Path::new(STATE_FILE).exists() {
            let mut file = std::fs::File::open(STATE_FILE).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            toml::from_str(&content).unwrap()
        } else {
            Self::default()
        }
    }

    pub fn current_farm_with_locked_rewards_address(&self) -> &Bech32Address {
        self.farm_with_locked_rewards
            .as_ref()
            .expect("no known farm with locked rewards contract, set first")
    }

    pub fn current_pair_address(&self) -> &Bech32Address {
        self.pair
            .as_ref()
            .expect("no known pair contract, set first")
    }

    pub fn _current_router_address(&self) -> &Bech32Address {
        self.router
            .as_ref()
            .expect("no known router contract, set first")
    }

    pub fn _current_farm_staking_address(&self) -> &Bech32Address {
        self.farm_staking
            .as_ref()
            .expect("no known farm staking contract, set first")
    }

    pub fn _current_farm_staking_proxy_address(&self) -> &Bech32Address {
        self.farm_staking_proxy
            .as_ref()
            .expect("no known farm staking proxy contract, set first")
    }

    pub fn _current_energy_factory_address(&self) -> &Bech32Address {
        self.energy_factory
            .as_ref()
            .expect("no known energy factory contract, set first")
    }

    pub fn first_token_id(&self) -> &String {
        self.first_token_id
            .as_ref()
            .expect("no knows first token id, set first")
    }

    pub fn second_token_id(&self) -> &String {
        self.second_token_id
            .as_ref()
            .expect("no knows second token id, set first")
    }
}

impl Drop for State {
    // Serializes state to file
    fn drop(&mut self) {
        let mut file = std::fs::File::create(STATE_FILE).unwrap();
        file.write_all(toml::to_string(self).unwrap().as_bytes())
            .unwrap();
    }
}
