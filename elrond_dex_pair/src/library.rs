imports!();
derive_imports!();

#[elrond_wasm_derive::module(LibraryModuleImpl)]
pub trait LibraryModule {

	fn calculate_k(&self, amount_a: BigUint, amount_b: BigUint) -> BigUint {
		amount_a * amount_b
	}

	#[view]
	fn quote(&self, amount_a: BigUint, reserve_a: BigUint, reserve_b: BigUint) -> BigUint {
		let amount_b = (amount_a * reserve_b) / reserve_a;

		amount_b
	}

	#[view(getAmountOut)]
	fn get_amount_out(&self, amount_in: BigUint, reserve_in: BigUint, reserve_out: BigUint) -> BigUint {
		let amount_in_with_fee = amount_in * BigUint::from(997u64);
		let numerator = amount_in_with_fee.clone() * reserve_out;
		let denominator = (reserve_in * BigUint::from(1000u64)) + amount_in_with_fee;

		let amount_out = numerator / denominator;

		amount_out
	}

	#[view(getAmountOutNoFee)]
	fn get_amount_out_no_fee(&self, amount_in: BigUint, reserve_in: BigUint, reserve_out: BigUint) -> BigUint {
		let numerator = amount_in.clone() * reserve_out;
		let denominator = reserve_in + amount_in;
		let amount_out = numerator / denominator;

		amount_out
	}

	#[view(getAmountIn)]
	fn get_amount_in(&self, amount_out: BigUint, reserve_in: BigUint, reserve_out: BigUint) -> BigUint {
		let numerator = (reserve_in * amount_out.clone()) * BigUint::from(1000u64);
		let denominator = (reserve_out - amount_out) * BigUint::from(997u64);

		let amount_in = (numerator / denominator) + BigUint::from(1u64);

		amount_in
	}

	fn get_fee_fixed_input(&self, amount_in: BigUint) -> BigUint {
		amount_in / BigUint::from(1000u64) 
	}

	fn get_fee_optimal_input(&self, amount_in_optimal: BigUint) -> BigUint {
		amount_in_optimal * BigUint::from(997u64) / BigUint::from(1000u64) / BigUint::from(999u64)
	}
}
