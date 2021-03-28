use elrond_dex_router::*;
use elrond_wasm_debug::*;

fn main() {
	let contract = RouterImpl::new(TxContext::dummy());
	print!("{}", abi_json::contract_abi(&contract));
}