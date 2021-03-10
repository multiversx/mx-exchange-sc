imports!();
derive_imports!();

#[elrond_wasm_derive::module(LibraryModuleImpl)]
pub trait LibraryModule {

	fn quote(&self, amount_a: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		let amount_b = (amount_a * reserves.1) / reserves.0;

		amount_b
	}

	fn get_amount_out(&self, amount_in: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		let amount_in_with_fee = amount_in * BigUint::from(997u64);
		let numerator = amount_in_with_fee.clone() * reserves.1;
		let denominator = (reserves.0 * BigUint::from(1000u64)) + amount_in_with_fee;

		let amount_out = numerator / denominator;

		amount_out
	}

	fn get_amount_in(&self, amount_out: BigUint, reserves: (BigUint, BigUint)) -> BigUint {
		let numerator = (reserves.0 * amount_out.clone()) * BigUint::from(1000u64);
		let denominator = (reserves.1 - amount_out) * BigUint::from(997u64);

		let amount_in = (numerator / denominator) + BigUint::from(1u64);

		amount_in
	}

	fn get_fee_fixed_input(&self, amount_in: BigUint) -> BigUint {
		amount_in / BigUint::from(1000u64) 
	}

	fn get_fee_optimal_input(&self, amount_in_optimal: BigUint) -> BigUint {
		amount_in_optimal * BigUint::from(997u64) / BigUint::from(1000u64) / BigUint::from(999u64)
	}

	fn minimum(&self, value_a: BigUint, value_b: BigUint) -> BigUint {
		if value_a <= value_b {
			return value_a;
		}

		return value_b;
	}

}