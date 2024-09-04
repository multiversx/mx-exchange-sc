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
    multisig: Option<Bech32Address>,
    energy_factory: Option<Bech32Address>,
    fees_collector: Option<Bech32Address>,
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

    pub fn current_energy_factory_address(&self) -> &Bech32Address {
        self.energy_factory
            .as_ref()
            .expect("no known energy factory contract, set first")
    }

    pub fn current_multisig_address(&self) -> &Bech32Address {
        self.multisig
            .as_ref()
            .expect("no known multisig contract, set first")
    }

    pub fn current_fees_collector_address(&self) -> &Bech32Address {
        self.fees_collector
            .as_ref()
            .expect("no known fees collector contract, set first")
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
