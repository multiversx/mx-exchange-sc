use elrond_dex_staking::*;
use elrond_wasm_debug::*;

fn main() {
	let contract = StakingImpl::new(TxContext::dummy());
	print!("{}", abi_json::contract_abi(&contract));
}