extern crate elrond_dex_factory;
use elrond_dex_factory::*;
use elrond_wasm_debug::*;

#[test]
fn test_add() {
	let adder = FactoryImpl::new(TxContext::dummy());

	adder.init(&RustBigInt::from(5));
	assert_eq!(RustBigInt::from(5), adder.get_sum());

	let _ = adder.add(&RustBigInt::from(7));
	assert_eq!(RustBigInt::from(12), adder.get_sum());

	let _ = adder.add(&RustBigInt::from(1));
	assert_eq!(RustBigInt::from(13), adder.get_sum());
}
