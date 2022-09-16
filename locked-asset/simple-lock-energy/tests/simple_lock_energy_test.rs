mod simple_lock_energy_setup;
use simple_lock_energy_setup::*;

#[test]
fn init_test() {
    let _ = SimpleLockEnergySetup::new(simple_lock_energy::contract_obj);
}
