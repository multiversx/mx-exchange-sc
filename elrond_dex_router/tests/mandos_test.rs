extern crate elrond_dex_router;
use elrond_dex_router::*;
use elrond_wasm::*;
use elrond_wasm_debug::*;

fn contract_map() -> ContractMap<TxContext> {
	let mut contract_map = ContractMap::new();
	contract_map.register_contract(
		"file:../output/elrond_dex_router.wasm",
		Box::new(|context| Box::new(RouterImpl::new(context))),
	);
	contract_map
}

#[test]
fn construct_code_test() {
	parse_execute_mandos("mandos/construct_code.scen.json", &contract_map());
}

#[test]
fn construct_code_test() {
	parse_execute_mandos("mandos/create_pair.scen.json", &contract_map());
}

#[test]
fn construct_code_test() {
	parse_execute_mandos("mandos/get_pair_views.scen.json", &contract_map());
}

#[test]
fn construct_code_test() {
	parse_execute_mandos("mandos/pause.scen.json", &contract_map());
}

#[test]
fn construct_code_test() {
	parse_execute_mandos("mandos/pause.scen.json", &contract_map());
}
