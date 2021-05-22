use elrond_wasm::*;
use elrond_wasm_debug::*;

#[allow(dead_code)]
fn contract_map() -> ContractMap<TxContext> {
    let mut contract_map = ContractMap::new();
    contract_map.register_contract(
        "file:../output/elrond_dex_router.wasm",
        Box::new(|context| Box::new(elrond_dex_router::contract_obj(context))),
    );
    contract_map
}

// #[test]
// fn create_pair_twice_test() {
//     parse_execute_mandos("mandos/create_pair_twice.scen.json", &contract_map());
// }

// #[test]
// fn get_pair_views_test() {
//     parse_execute_mandos("mandos/get_pair_views.scen.json", &contract_map());
// }

// #[test]
// fn pause_test() {
//     parse_execute_mandos("mandos/pause.scen.json", &contract_map());
// }

// #[test]
// fn resume_test() {
//     parse_execute_mandos("mandos/resume.scen.json", &contract_map());
// }

// #[test]
// fn set_staking_info_test() {
//     parse_execute_mandos("mandos/set_staking_info.scen.json", &contract_map());
// }
