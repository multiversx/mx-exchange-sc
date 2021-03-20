use elrond_dex_pair::*;
use elrond_wasm_debug::*;

fn main() {
	let contract = PairImpl::new(TxContext::dummy());
	print!("{}", abi_json::contract_abi(&contract));
}