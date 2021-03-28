#![no_std]

imports!();
derive_imports!();

pub mod liquidity_pool;
pub mod library;
pub mod fee;

pub use crate::liquidity_pool::*;
pub use crate::library::*;
pub use crate::fee::*;

#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[module(LiquidityPoolModuleImpl)]
	fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

	#[module(LibraryModuleImpl)]
	fn library(&self) -> LibraryModuleImpl<T, BigInt, BigUint>;

	#[module(FeeModuleImpl)]
	fn fee(&self) -> FeeModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(
		&self,
		token_a_name: TokenIdentifier,
		token_b_name: TokenIdentifier,
		router_address: Address) {
		
		self.router_address().set(&router_address);
		self.liquidity_pool().token_a_name().set(&token_a_name);
		self.liquidity_pool().token_b_name().set(&token_b_name);

		self.fee().state().set(&false);
	}

	#[payable("*")]
	#[endpoint(acceptEsdtPayment)]
	fn accept_payment_endpoint(
		&self,
		#[payment_token] token: TokenIdentifier,
		#[payment] payment: BigUint,
	) -> SCResult<()> {

		require!(payment > 0, "PAIR: Funds transfer must be a positive number");
		if token != self.liquidity_pool().token_a_name().get() && token != self.liquidity_pool().token_b_name().get() {
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

		if self.lp_token_identifier().is_empty() {
			return sc_error!("Lp token not issued");
		}

		let caller = self.get_caller();
		let expected_token_a_name = self.liquidity_pool().token_a_name().get();
		let expected_token_b_name = self.liquidity_pool().token_b_name().get();
		let mut temporary_amount_a_desired = self.get_temporary_funds(&caller, &expected_token_a_name);
		let mut temporary_amount_b_desired = self.get_temporary_funds(&caller, &expected_token_b_name);

		require!(temporary_amount_a_desired > 0, "PAIR: NO AVAILABLE TOKEN A FUNDS");
		require!(temporary_amount_b_desired > 0, "PAIR: NO AVAILABLE TOKEN B FUNDS");
		require!(amount_a_desired <= temporary_amount_a_desired, "PAIR: INSSUFICIENT TOKEN A FUNDS TO ADD");
		require!(amount_b_desired <= temporary_amount_b_desired, "PAIR: INSSUFICIENT TOKEN B FUNDS TO ADD");

		let amounts = sc_try!(
			self.liquidity_pool()._add_liquidity(
				amount_a_desired.clone(), 
				amount_b_desired.clone(), 
				amount_a_min, 
				amount_b_min
			)
		);
		let amount_a = amounts.0;
		let amount_b = amounts.1;

		let liquidity = sc_try!(self.liquidity_pool().mint(amount_a.clone(), amount_b.clone()));

		self.send().esdt_local_mint(
			self.get_gas_left(),
			self.lp_token_identifier().get().as_esdt_identifier(),
			&liquidity,
		);

		self.send().direct_esdt_via_transf_exec(
			&self.get_caller(),
			self.lp_token_identifier().get().as_esdt_identifier(),
			&liquidity,
			&[]
		);

		let mut total_supply = self.liquidity_pool().total_supply().get();
		total_supply += liquidity.clone();
		self.liquidity_pool().total_supply().set(&total_supply);

		temporary_amount_a_desired -= amount_a;
		temporary_amount_b_desired -= amount_b;
		self.set_temporary_funds(&caller, &expected_token_a_name, &temporary_amount_a_desired);
		self.set_temporary_funds(&caller, &expected_token_b_name, &temporary_amount_b_desired);

		Ok(())
	}

	fn reclaim_temporary_token(&self, token: &TokenIdentifier) {
		let caller = self.get_caller();
		let amount = self.get_temporary_funds(&caller, token);
		if amount > 0 {
			self.send().direct_esdt_via_transf_exec(&caller, token.as_esdt_identifier(), &amount, &[]);
			self.clear_temporary_funds(&caller, token);
		}
	}

	#[endpoint(reclaimTemporaryFunds)]
	fn reclaim_temporary_funds(&self) -> SCResult<()> {
		self.reclaim_temporary_token(
			&self.liquidity_pool().token_a_name().get()
		);
		self.reclaim_temporary_token(
			&self.liquidity_pool().token_b_name().get()
		);
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

		if self.lp_token_identifier().is_empty() {
			return sc_error!("Lp token not issued");
		}
	
		let caller = self.get_caller();
		let expected_liquidity_token = self.lp_token_identifier().get();
		require!(liquidity_token == expected_liquidity_token, "PAIR: Wrong liquidity token");
		
		let amounts = sc_try!(self.liquidity_pool().burn(liquidity.clone(), amount_a_min, amount_b_min));
		let amount_a = amounts.0;
		let amount_b = amounts.1;
		let token_a = self.liquidity_pool().token_a_name().get();
		let token_b = self.liquidity_pool().token_b_name().get();
		let mut total_supply = self.liquidity_pool().total_supply().get();
		total_supply -= liquidity.clone();

		self.send().direct_esdt_via_transf_exec(&caller, token_a.as_esdt_identifier(), &amount_a, &[]);
		self.send().direct_esdt_via_transf_exec(&caller, token_b.as_esdt_identifier(), &amount_b, &[]);
		self.liquidity_pool().total_supply().set(&total_supply);

		self.send().esdt_local_burn(
			self.get_gas_left(),
			expected_liquidity_token.as_esdt_identifier(),
			&liquidity,
		);

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
		if token_in != self.liquidity_pool().token_a_name().get() && token_in != self.liquidity_pool().token_b_name().get() {
			return sc_error!("Pair: Invalid token");
		}
		require!(amount_out_min > 0, "Invalid amount_out_min");
		if token_out != self.liquidity_pool().token_a_name().get() && token_out != self.liquidity_pool().token_b_name().get() {
			return sc_error!("Pair: Invalid token");
		}

		let mut balance_token_out = self.liquidity_pool().get_pair_reserve(&token_out);
		require!(balance_token_out > amount_out_min, "Insufficient balance for token out");

		let mut balance_token_in = self.liquidity_pool().get_pair_reserve(&token_in);
		let amount_out_optimal = self.library().get_amount_out(
			amount_in.clone(), 
			balance_token_in.clone(), 
			balance_token_out.clone()
		);
		require!(amount_out_optimal >= amount_out_min, "Insufficient liquidity");
		require!(balance_token_out > amount_out_optimal, "Insufficient balance");

		self.send().direct_esdt_via_transf_exec(&self.get_caller(), token_out.as_esdt_identifier(), &amount_out_optimal, &[]);

		let mut fee_amount = BigUint::zero();
		let mut amount_in_after_fee = amount_in.clone();
		if self.fee().state().get() {
			fee_amount = self.library().get_fee_fixed_input(amount_in.clone());
			amount_in_after_fee -= fee_amount.clone();
		}

		balance_token_in += amount_in_after_fee;
		balance_token_out -= amount_out_optimal;

		self.liquidity_pool().set_pair_reserve(&token_in, &balance_token_in);
		self.liquidity_pool().set_pair_reserve(&token_out, &balance_token_out);

		//The transaction was made. We are left with $(fee) of $(token_in) as fee.
		if self.fee().state().get() {
			self.send_fee(token_in, fee_amount);
		}

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
		if token_in != self.liquidity_pool().token_a_name().get() && token_in != self.liquidity_pool().token_b_name().get() {
			return sc_error!("Pair: Invalid token");
		}
		require!(amount_out > 0, "Invalid amount_out_min");
		if token_out != self.liquidity_pool().token_a_name().get() && token_out != self.liquidity_pool().token_b_name().get() {
			return sc_error!("Pair: Invalid token");
		}

		let mut balance_token_out = self.liquidity_pool().get_pair_reserve(&token_out);
		require!(balance_token_out > amount_out, "Insufficient balance for token out");

		let mut balance_token_in = self.liquidity_pool().get_pair_reserve(&token_in);
		let amount_in_optimal = self.library().get_amount_in(
			amount_out.clone(), 
			balance_token_in.clone(), 
			balance_token_out.clone()
		);
		require!(amount_in_optimal <= amount_in_max, "Insufficient liquidity");

		self.send().direct_esdt_via_transf_exec(&self.get_caller(), token_out.as_esdt_identifier(), &amount_out, &[]);
		let residuum = amount_in_max.clone() - amount_in_optimal.clone();
		if residuum != BigUint::from(0u64) {
			self.send().direct_esdt_via_transf_exec(&self.get_caller(), token_in.as_esdt_identifier(), &residuum, &[]);
		}

		let mut fee_amount = BigUint::zero();
		let mut amount_in_optimal_after_fee = amount_in_optimal.clone();
		if self.fee().state().get() {
			fee_amount = self.library().get_fee_optimal_input(amount_in_optimal.clone());
			amount_in_optimal_after_fee -= fee_amount.clone();
		}
		require!(balance_token_out > amount_out, "Insufficient balance");

		balance_token_in += amount_in_optimal_after_fee;
		balance_token_out -= amount_out;
		self.liquidity_pool().set_pair_reserve(&token_in, &balance_token_in);
		self.liquidity_pool().set_pair_reserve(&token_out, &balance_token_out);

		//The transaction was made. We are left with $(fee) of $(token_in) as fee.
		if self.fee().state().get() {
			self.send_fee(token_in, fee_amount);
		}

		Ok(())
  }

	#[endpoint]
	fn set_fee_on_endpoint(
		&self, 
		enabled: bool, 
		fee_to_address: Address, 
		fee_token: TokenIdentifier
	) {
		if self.get_caller() == self.router_address().get() {
			self.fee().state().set(&enabled);
			self.fee().address().set(&fee_to_address);
			self.fee().token_identifier().set(&fee_token);
		}
	}

	fn send_fee(
		&self, 
		fee_token: TokenIdentifier, 
		fee_amount: BigUint
	) {
		if fee_amount == BigUint::zero() {
			return;
		}

		let fee_token_requested = self.fee().token_identifier().get();
		let token_a = self.liquidity_pool().token_a_name().get();
		let token_b = self.liquidity_pool().token_b_name().get();
		let mut to_send = BigUint::zero();

		if fee_token_requested == fee_token  {
			// Luckily no conversion is required.
			to_send = fee_amount.clone();
		}
		else if fee_token_requested == token_a && fee_token == token_b {
			// Fees are in form of token_b. Need to convert them to token_a.
			let mut balance_token_b = self.liquidity_pool().get_pair_reserve(&token_b);
			let mut balance_token_a = self.liquidity_pool().get_pair_reserve(&token_a);
			let fee_amount_swap = self.library().get_amount_out_no_fee(
				fee_amount.clone(), 
				balance_token_b.clone(), 
				balance_token_a.clone()
			);

			if balance_token_a > fee_amount_swap && fee_amount_swap > BigUint::zero() {
				//There are enough tokens for swapping.
				balance_token_a -= fee_amount_swap.clone();
				balance_token_b += fee_amount.clone();
				self.liquidity_pool().set_pair_reserve(&token_a, &balance_token_a);
				self.liquidity_pool().set_pair_reserve(&token_b, &balance_token_b);
				to_send = fee_amount_swap;
			}
		}
		else if fee_token_requested == token_b && fee_token == token_a {
			// Fees are in form of token_a. Need to convert them to token_b.
			let mut balance_token_a = self.liquidity_pool().get_pair_reserve(&token_a);
			let mut balance_token_b = self.liquidity_pool().get_pair_reserve(&token_b);
			let fee_amount_swap = self.library().get_amount_out_no_fee(
				fee_amount.clone(), 
				balance_token_a.clone(), 
				balance_token_b.clone()
			);

			if balance_token_b > fee_amount_swap && fee_amount_swap > BigUint::zero() {
				//There are enough tokens for swapping.
				balance_token_b -= fee_amount_swap.clone();
				balance_token_a += fee_amount.clone();
				self.liquidity_pool().set_pair_reserve(&token_a, &balance_token_a);
				self.liquidity_pool().set_pair_reserve(&token_b, &balance_token_b);
				to_send = fee_amount_swap;
			}
		}

		if to_send > BigUint::zero() {
			self.send().direct_esdt_via_transf_exec(
				&self.fee().address().get(),
				self.fee().token_identifier().get().as_esdt_identifier(),
				&to_send,
				&[]
			);
		}
		else {
			// Either swap failed or requested token identifier differs from both token_a and token_b.
			// Reinject them into liquidity pool.
			let mut reserve = self.liquidity_pool().get_pair_reserve(&fee_token);
			reserve += fee_amount;
			self.liquidity_pool().set_pair_reserve(&fee_token, &reserve);
		}
	}

	#[endpoint]
	fn set_lp_token_identifier_endpoint(&self, token_identifier: TokenIdentifier) -> SCResult<()>{
		let caller = self.get_caller();
		require!(caller == self.router_address().get(), "PAIR: Permission Denied");
		if self.lp_token_identifier().is_empty() {
			self.lp_token_identifier().set(&token_identifier);
		}

		Ok(())
	}

	#[view]
	fn get_lp_token_identifier_endpoint(&self) -> TokenIdentifier {
		self.lp_token_identifier().get()
	}


	#[view]
	fn get_tokens_for_given_position(
		&self, 
		liquidity: BigUint
	) -> ((TokenIdentifier, BigUint), (TokenIdentifier, BigUint)) {
		self.liquidity_pool().get_tokens_for_given_position(liquidity)
	}

	// Temporary Storage
	#[view(getTemporaryFunds)]
	#[storage_get("funds")]
	fn get_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier) -> BigUint;

	#[storage_set("funds")]
	fn set_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier, amount: &BigUint);

	#[storage_clear("funds")]
	fn clear_temporary_funds(&self, caller: &Address, token_identifier: &TokenIdentifier);

	#[storage_mapper("lpTokenIdentifier")]
	fn lp_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

	#[view(getRouterAddress)]
	#[storage_mapper("router_address")]
	fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;
}
