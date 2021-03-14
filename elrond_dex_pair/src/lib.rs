#![no_std]

imports!();
derive_imports!();

pub mod liquidity_pool;
pub mod library;

pub use crate::liquidity_pool::*;
pub use crate::library::*;

use elrond_wasm::HexCallDataSerializer;

const ESDT_DECIMALS: u8 = 18;
const ESDT_ISSUE_STRING: &[u8] = b"issue";
const ESDT_ISSUE_COST: u64 = 5000000000000000000; // 5 eGLD
const LP_TOKEN_INITIAL_SUPPLY: u32 = u32::MAX; //Can be any u64 != 0
// erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] = [
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
	0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff,
];

#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[module(LiquidityPoolModuleImpl)]
	fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

	#[module(LibraryModuleImpl)]
	fn library(&self) -> LibraryModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(
		&self,
		token_a_name: TokenIdentifier,
		token_b_name: TokenIdentifier,
		router_address: Address) {
		
		self.set_router_address(&router_address);
		self.liquidity_pool().set_token_a_name(&token_a_name);
		self.liquidity_pool().set_token_b_name(&token_b_name);

		self.set_fee_on(false);
		self.set_fee_reserve(&token_a_name, &BigUint::from(0u64));
		self.set_fee_reserve(&token_b_name, &BigUint::from(0u64));
	}

	#[payable("*")]
	#[endpoint(acceptEsdtPayment)]
	fn accept_payment_endpoint(
		&self,
		#[payment_token] token: TokenIdentifier,
		#[payment] payment: BigUint,
	) -> SCResult<()> {

		require!(payment > 0, "PAIR: Funds transfer must be a positive number");
		if token != self.liquidity_pool().get_token_a_name() && token != self.liquidity_pool().get_token_b_name() {
			return sc_error!("PAIR: INVALID TOKEN");
		}

		let caller = self.get_caller();
		let mut temporary_funds = self.get_temporary_funds(&caller, &token);
		temporary_funds += payment;
		self.set_temporary_funds(&caller, &token, &temporary_funds);

		Ok(())
	}

	#[endpoint(addLiquidity)]
	fn add_liquidity_endpoint(
		&self,
		amount_a_desired: BigUint,
		amount_b_desired: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint) -> SCResult<()> {

		require!(amount_a_desired > 0, "PAIR: INSSUFICIENT TOKEN A FUNDS SENT");
		require!(amount_b_desired > 0, "PAIR: INSSUFICIENT TOKEN B FUNDS SENT");

		if self.is_empty_lp_token_identifier() {
			return sc_error!("Lp token not issued");
		}

		let caller = self.get_caller();
		let expected_token_a_name = self.liquidity_pool().get_token_a_name();
		let expected_token_b_name = self.liquidity_pool().get_token_b_name();
		let mut temporary_amount_a_desired = self.get_temporary_funds(&caller, &expected_token_a_name);
		let mut temporary_amount_b_desired = self.get_temporary_funds(&caller, &expected_token_b_name);

		require!(temporary_amount_a_desired > 0, "PAIR: NO AVAILABLE TOKEN A FUNDS");
		require!(temporary_amount_b_desired > 0, "PAIR: NO AVAILABLE TOKEN B FUNDS");
		require!(amount_a_desired <= temporary_amount_a_desired, "PAIR: INSSUFICIENT TOKEN A FUNDS TO ADD");
		require!(amount_b_desired <= temporary_amount_b_desired, "PAIR: INSSUFICIENT TOKEN B FUNDS TO ADD");

		let amount_a: BigUint;
		let amount_b: BigUint;
		let result = self.liquidity_pool()._add_liquidity(amount_a_desired.clone(), amount_b_desired.clone(), amount_a_min, amount_b_min);

		match result {
			SCResult::Ok(amounts) => {
				amount_a = amounts.0;
				amount_b = amounts.1;
			},
			SCResult::Err(err) => {
				return sc_error!(err);
			},
		};

		let result = self.liquidity_pool().mint(
			amount_a.clone(),
			amount_b.clone(),
		);

		match result {
			SCResult::Ok(liquidity) => {
				self.send().direct_esdt_via_transf_exec(
					&self.get_caller(),
					self.get_lp_token_identifier().as_slice(),
					&liquidity,
					&[],
				);

				let mut total_supply = self.liquidity_pool().get_total_supply();
				total_supply += liquidity.clone();
				self.liquidity_pool().set_total_supply(&total_supply);

				temporary_amount_a_desired -= amount_a;
				temporary_amount_b_desired -= amount_b;
				self.set_temporary_funds(&caller, &expected_token_a_name, &temporary_amount_a_desired);
				self.set_temporary_funds(&caller, &expected_token_b_name, &temporary_amount_b_desired);
			},
			SCResult::Err(err) => {
				return sc_error!(err);
			}
		};
		Ok(())
	}

	#[endpoint(reclaimTemporaryFunds)]
	fn reclaim_temporary_funds(&self) -> SCResult<()> {

		let caller = self.get_caller();
		let token_a = self.liquidity_pool().get_token_a_name();
		let token_b = self.liquidity_pool().get_token_b_name();
		let amount_a = self.get_temporary_funds(&caller, &token_a);
		let amount_b = self.get_temporary_funds(&caller, &token_b);
		
		if amount_a > 0 {
			self.send().direct_esdt_via_transf_exec(&caller, token_a.as_slice(), &amount_a, &[]);
			self.clear_temporary_funds(&caller, &token_a);
		}
		if amount_b > 0 {
			self.send().direct_esdt_via_transf_exec(&caller, token_b.as_slice(), &amount_b, &[]);
			self.clear_temporary_funds(&caller, &token_b);
		}

		Ok(())
	}

	#[payable("*")]
	#[endpoint(removeLiquidity)]
	fn remove_liquidity(
		&self,
		#[payment_token] liquidity_token: TokenIdentifier,
		#[payment] liquidity: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint) -> SCResult<()> {

		if self.is_empty_lp_token_identifier() {
			return sc_error!("Lp token not issued");
		}
	
		let caller = self.get_caller();
		let expected_liquidity_token = self.get_lp_token_identifier();
		require!(liquidity_token == expected_liquidity_token, "PAIR: Wrong liquidity token");
		
		let result = self.liquidity_pool().burn(liquidity.clone(), amount_a_min, amount_b_min);
		
		match result {
			SCResult::Ok(amounts) => {
				let token_a = self.liquidity_pool().get_token_a_name();
				let token_b = self.liquidity_pool().get_token_b_name();
				let (amount_a, amount_b) = amounts;

				self.send().direct_esdt_via_transf_exec(&caller, token_a.as_slice(), &amount_a, &[]);
				self.send().direct_esdt_via_transf_exec(&caller, token_b.as_slice(), &amount_b, &[]);

				let mut total_supply = self.liquidity_pool().get_total_supply();
				total_supply -= liquidity.clone();
				self.liquidity_pool().set_total_supply(&total_supply);
			},
			SCResult::Err(err) => {
				return sc_error!(err);
			}
		}
		Ok(())
	}

	#[payable("*")]
	#[endpoint(swapTokensFixedInput)]
	fn swap_tokens_fixed_input(
		&self,
		#[payment_token] token_in: TokenIdentifier,
		#[payment] amount_in: BigUint,
		token_out: TokenIdentifier,
		amount_out_min: BigUint
	) -> SCResult<()> {

		if token_in == token_out {
			return sc_error!("Swap with same token");
		}
		require!(amount_in > 0, "Invalid amount_in");
		if token_in != self.liquidity_pool().get_token_a_name() && token_in != self.liquidity_pool().get_token_b_name() {
			return sc_error!("Pair: Invalid token");
		}
		require!(amount_out_min > 0, "Invalid amount_out_min");
		if token_out != self.liquidity_pool().get_token_a_name() && token_out != self.liquidity_pool().get_token_b_name() {
			return sc_error!("Pair: Invalid token");
		}

		let mut balance_token_out = self.liquidity_pool().get_pair_reserve(&token_out);
		require!(balance_token_out > amount_out_min, "Insufficient balance for token out");

		let mut balance_token_in = self.liquidity_pool().get_pair_reserve(&token_in);
		let tmp = (balance_token_in.clone(), balance_token_out.clone());
		let amount_out_optimal = self.library().get_amount_out(amount_in.clone(), tmp);
		require!(amount_out_optimal >= amount_out_min, "Insufficient liquidity");

		self.send().direct_esdt_via_transf_exec(&self.get_caller(), token_out.as_slice(), &amount_out_optimal, &[]);

		let mut amount_in_after_fee = amount_in.clone();
		if self.get_fee_on() {
			let fee = self.library().get_fee_fixed_input(amount_in.clone());
			let mut fee_amount = self.get_fee_reserve(&token_in);

			fee_amount += fee.clone();
			self.set_fee_reserve(&token_in, &fee_amount);
			amount_in_after_fee -= fee;
		}

		balance_token_in += amount_in_after_fee;
		balance_token_out -= amount_out_optimal;
		self.liquidity_pool().set_pair_reserve(&token_in, &balance_token_in);
		self.liquidity_pool().set_pair_reserve(&token_out, &balance_token_out);

		Ok(())
	}

	#[payable("*")]
	#[endpoint(swapTokensFixedOutput)]
	fn swap_tokens_fixed_output(
		&self,
		#[payment_token] token_in: TokenIdentifier,
		#[payment] amount_in_max: BigUint,
		token_out: TokenIdentifier,
		amount_out: BigUint
	) -> SCResult<()> {

		if token_in == token_out {
			return sc_error!("Swap with same token");
		}
		require!(amount_in_max > 0, "Invalid amount_in");
		if token_in != self.liquidity_pool().get_token_a_name() && token_in != self.liquidity_pool().get_token_b_name() {
			return sc_error!("Pair: Invalid token");
		}
		require!(amount_out > 0, "Invalid amount_out_min");
		if token_out != self.liquidity_pool().get_token_a_name() && token_out != self.liquidity_pool().get_token_b_name() {
			return sc_error!("Pair: Invalid token");
		}

		let mut balance_token_out = self.liquidity_pool().get_pair_reserve(&token_out);
		require!(balance_token_out > amount_out, "Insufficient balance for token out");

		let mut balance_token_in = self.liquidity_pool().get_pair_reserve(&token_in);
		let tmp = (balance_token_in.clone(), balance_token_out.clone());
		let amount_in_optimal = self.library().get_amount_in(amount_out.clone(), tmp);
		require!(amount_in_optimal <= amount_in_max, "Insufficient liquidity");

		self.send().direct_esdt_via_transf_exec(&self.get_caller(), token_out.as_slice(), &amount_out, &[]);
		let residuum = amount_in_max.clone() - amount_in_optimal.clone();
		if residuum > BigUint::from(0u64) {
			self.send().direct_esdt_via_transf_exec(&self.get_caller(), token_in.as_slice(), &residuum, &[]);
		}

		let mut amount_in_optimal_after_fee = amount_in_optimal.clone();
		if self.get_fee_on() {
			let fee = self.library().get_fee_optimal_input(amount_in_optimal.clone());
			let mut fee_amount = self.get_fee_reserve(&token_in);

			fee_amount += fee.clone();
			self.set_fee_reserve(&token_in, &fee_amount);
			amount_in_optimal_after_fee -= fee;
		}

		balance_token_in += amount_in_optimal_after_fee;
		balance_token_out -= amount_out;
		self.liquidity_pool().set_pair_reserve(&token_in, &balance_token_in);
		self.liquidity_pool().set_pair_reserve(&token_out, &balance_token_out);

		Ok(())
	}

	#[payable("EGLD")]
	#[endpoint(issueLpToken)]
	fn issue_token(
		&self,
		tp_token_display_name: BoxedBytes,
		tp_token_ticker: BoxedBytes,
		#[payment] payment: BigUint
	) -> SCResult<()> {

		if self.is_empty_lp_token_identifier() == false {
			return sc_error!("Already issued");
		}
		if payment != BigUint::from(ESDT_ISSUE_COST) {
			return sc_error!("Should pay exactly 5 EGLD");
		}

		let tp_token_initial_supply = BigUint::from(LP_TOKEN_INITIAL_SUPPLY);
		let mut serializer = HexCallDataSerializer::new(ESDT_ISSUE_STRING);
		serializer.push_argument_bytes(tp_token_display_name.as_slice());
		serializer.push_argument_bytes(tp_token_ticker.as_slice());
		serializer.push_argument_bytes(&tp_token_initial_supply.to_bytes_be());
		serializer.push_argument_bytes(&[ESDT_DECIMALS]);

		serializer.push_argument_bytes(&b"canFreeze"[..]);
		serializer.push_argument_bytes(&b"false"[..]);

		serializer.push_argument_bytes(&b"canWipe"[..]);
		serializer.push_argument_bytes(&b"false"[..]);

		serializer.push_argument_bytes(&b"canPause"[..]);
		serializer.push_argument_bytes(&b"false"[..]);

		serializer.push_argument_bytes(&b"canMint"[..]);
		serializer.push_argument_bytes(&b"true"[..]);

		serializer.push_argument_bytes(&b"canBurn"[..]);
		serializer.push_argument_bytes(&b"true"[..]);

		serializer.push_argument_bytes(&b"canChangeOwner"[..]);
		serializer.push_argument_bytes(&b"false"[..]);

		serializer.push_argument_bytes(&b"canUpgrade"[..]);
		serializer.push_argument_bytes(&b"true"[..]);

		self.send().async_call_raw(
			&Address::from(ESDT_SYSTEM_SC_ADDRESS_ARRAY),
			&BigUint::from(ESDT_ISSUE_COST),
			serializer.as_slice(),
		);
	}

	#[callback_raw]
	fn callback_raw(&self, #[var_args] result: AsyncCallResult<VarArgs<BoxedBytes>>) {
		let success = match result {
			AsyncCallResult::Ok(_) => true,
			AsyncCallResult::Err(_) => false,
		};

		if success && self.is_empty_lp_token_identifier() {
			self.set_lp_token_identifier(&self.call_value().token());
		}
	}

	#[endpoint]
	fn set_fee_on_endpoint(&self, enabled: bool) {
		if self.get_caller() == self.get_router_address() {
			self.set_fee_on(enabled);
		}
	}

	// Temporary Storage
	#[view(getTemporaryFunds)]
	#[storage_get("funds")]
	fn get_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier) -> BigUint;

	#[storage_set("funds")]
	fn set_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);

	#[storage_clear("funds")]
	fn clear_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier);


	#[view(getLpTokenIdentifier)]
	#[storage_get("lpTokenIdentifier")]
	fn get_lp_token_identifier(&self) -> TokenIdentifier;

	#[storage_set("lpTokenIdentifier")]
	fn set_lp_token_identifier(&self, token_identifier: &TokenIdentifier);

	#[storage_is_empty("lpTokenIdentifier")]
	fn is_empty_lp_token_identifier(&self) -> bool;

	#[storage_get("router_address")]
	fn get_router_address(&self) -> Address;

	#[storage_set("router_address")]
	fn set_router_address(&self, address: &Address);


	#[view(getFeeOn)]
	#[storage_get("fee_on")]
	fn get_fee_on(&self) -> bool;

	#[storage_set("fee_on")]
	fn set_fee_on(&self, enabled: bool);

	#[view(getFeeReserve)]
	#[storage_get("feeReserve")]
	fn get_fee_reserve(&self, token: &TokenIdentifier) -> BigUint;

	#[storage_set("feeReserve")]
	fn set_fee_reserve(&self, token: &TokenIdentifier, balance: &BigUint);
}
