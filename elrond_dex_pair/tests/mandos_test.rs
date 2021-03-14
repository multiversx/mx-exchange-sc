extern crate elrond_dex_pair;
use elrond_dex_pair::*;
use elrond_wasm::*;
use elrond_wasm_debug::*;

fn contract_map() -> ContractMap<TxContext> {
	let mut contract_map = ContractMap::new();
	contract_map.register_contract(
		"file:../output/elrond_dex_pair.wasm",
		Box::new(|context| Box::new(PairImpl::new(context))),
	);
	contract_map
}

#[test]
fn accept_esdt_payment_test() {
	parse_execute_mandos("mandos/accept_esdt_payment.scen.json", &contract_map());
}

#[test]
fn reclaim_temporary_funds_test() {
	parse_execute_mandos("mandos/reclaim_temporary_funds.scen.json", &contract_map());
}

#[test]
fn add_liquidity_test() {
	parse_execute_mandos("mandos/add_liquidity.scen.json", &contract_map());
}

#[test]
fn remove_liquidity_test() {
	parse_execute_mandos("mandos/remove_liquidity.scen.json", &contract_map());
}

#[test]
fn swap_fixed_input_test() {
	parse_execute_mandos("mandos/swap_fixed_input.scen.json", &contract_map());
}

#[test]
fn swap_fixed_output_test() {
	parse_execute_mandos("mandos/swap_fixed_output.scen.json", &contract_map());
}
