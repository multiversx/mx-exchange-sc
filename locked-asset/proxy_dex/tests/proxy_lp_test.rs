mod proxy_dex_test_setup;

use proxy_dex_test_setup::*;

#[test]
fn setup_test() {
    let _ = ProxySetup::new(
        proxy_dex::contract_obj,
        pair::contract_obj,
        farm::contract_obj,
        simple_lock_energy::contract_obj,
    );
}
