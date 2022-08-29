use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, LineWriter, Write},
};

static DEFAULT_ADDRESS_EXPR: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000000";

pub struct ContractAddressesRaw {
    pub farm_address_expr: String,
    pub energy_factory_address_expr: String,
}

impl Default for ContractAddressesRaw {
    fn default() -> Self {
        ContractAddressesRaw {
            farm_address_expr: DEFAULT_ADDRESS_EXPR.to_string(),
            energy_factory_address_expr: DEFAULT_ADDRESS_EXPR.to_string(),
        }
    }
}

impl ContractAddressesRaw {
    pub fn new_from_file(file_path: String) -> Self {
        let file = match File::open(file_path) {
            Ok(f) => f,
            Err(_) => return Self::default(),
        };
        let mut reader = BufReader::new(file);

        let mut farm_address_expr = String::new();
        if reader.read_line(&mut farm_address_expr).is_err() {
            farm_address_expr = DEFAULT_ADDRESS_EXPR.to_string();
        } else {
            // remove the "\n" character
            farm_address_expr.remove(farm_address_expr.len() - 1);
        }

        let mut energy_factory_address_expr = String::new();
        if reader.read_line(&mut energy_factory_address_expr).is_err() {
            energy_factory_address_expr = DEFAULT_ADDRESS_EXPR.to_string();
        };

        ContractAddressesRaw {
            farm_address_expr,
            energy_factory_address_expr,
        }
    }

    pub fn save_to_file(self, file_path: String) {
        let file = File::create(file_path).unwrap();
        let mut writer = LineWriter::new(file);

        writer.write_all(self.farm_address_expr.as_bytes()).unwrap();
        writer.write_all(b"\n").unwrap();
        writer
            .write_all(self.energy_factory_address_expr.as_bytes())
            .unwrap();
    }
}
