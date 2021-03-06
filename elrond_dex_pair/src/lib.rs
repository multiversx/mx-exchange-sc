#![no_std]

imports!();
derive_imports!();

pub mod liquidity_supply;
pub mod liquidity_pool;

pub use crate::liquidity_supply::*;
pub use crate::liquidity_pool::*;

use elrond_wasm::HexCallDataSerializer;

const ESDT_DECIMALS: u8 = 18;
const ESDT_ISSUE_STRING: &[u8] = b"issue";
const ESDT_ISSUE_COST: u64 = 5000000000000000000; // 5 eGLD
const LP_TOKEN_INITIAL_SUPPLY: u64 = ESDT_ISSUE_COST; //Can be any u64 != 0
// erd1qqqqqqqqqqqqqqqpqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqzllls8a5w6u
const ESDT_SYSTEM_SC_ADDRESS_ARRAY: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0xff, 0xff,
];

#[elrond_wasm_derive::contract(PairImpl)]
pub trait Pair {

	#[module(LiquiditySupplyModuleImpl)]
    fn supply(&self) -> LiquiditySupplyModuleImpl<T, BigInt, BigUint>;

	#[module(LiquidityPoolModuleImpl)]
    fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self, token_a_name: TokenIdentifier, token_b_name: TokenIdentifier, router_address: Address) {
		self.liquidity_pool().set_token_a_name(&token_a_name);
		self.liquidity_pool().set_token_b_name(&token_b_name);
		self.set_router_address(&router_address);
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
	fn add_liquidity_endpoint(&self) -> SCResult<()> {

		let caller = self.get_caller();
		let expected_token_a_name = self.liquidity_pool().get_token_a_name();
		let expected_token_b_name = self.liquidity_pool().get_token_b_name();
		let temporary_funds_amount_a = self.get_temporary_funds(&caller, &expected_token_a_name);
		let temporary_funds_amount_b = self.get_temporary_funds(&caller, &expected_token_b_name);

		require!(temporary_funds_amount_a > 0, "PAIR: NO AVAILABLE TOKEN A FUNDS");
		require!(temporary_funds_amount_b > 0, "PAIR: NO AVAILABLE TOKEN B FUNDS");
		
		let result = self.liquidity_pool().add_liquidity(
			temporary_funds_amount_a,
			temporary_funds_amount_b,
		);

		match result {
			SCResult::Ok(()) => {
				let caller = self.get_caller();
				self.clear_temporary_funds(&caller, &expected_token_a_name);
				self.clear_temporary_funds(&caller, &expected_token_b_name);
		Ok(())
			},
			SCResult::Err(err) => {
				// TODO: transfer temporary funds back to caller
				sc_error!(err)
			}
		}
	}


	#[endpoint]
	fn send_tokens_on_swap_success(&self,
		address: Address,
		token_in: TokenIdentifier,
		amount_in: BigUint,
		token_out: TokenIdentifier,
		amount_out: BigUint) -> SCResult<()> {

		require!(amount_in > 0, "Invalid tokens amount specified");
		require!(amount_out > 0, "Invalid tokens amount specified");

		let expected_token_a_name = self.liquidity_pool().get_token_a_name();
		let expected_token_b_name = self.liquidity_pool().get_token_b_name();
		require!(token_in == expected_token_a_name, "Wrong token a identifier");
		require!(token_out == expected_token_b_name, "Wrong token b identifier");


		//TODO: Check if amount_out is available. If not, send back what was received.
		self.send().direct_esdt_via_transf_exec(&address, token_out.as_slice(), &amount_out, &[]);

		let mut reserve_a = self.liquidity_pool().get_pair_reserve(&token_in);
		let mut reserve_b = self.liquidity_pool().get_pair_reserve(&token_out);
		reserve_a += amount_in;
		reserve_b -= amount_out;
		self.liquidity_pool().set_pair_reserve(&token_in, &reserve_a);
		self.liquidity_pool().set_pair_reserve(&token_out, &reserve_b);

		Ok(())
	}

	#[endpoint]
	fn remove_liquidity(&self, user_address: Address,
		actual_token_a_name: TokenIdentifier,
		actual_token_b_name: TokenIdentifier,
		liquidity: BigUint,
		amount_a_min: BigUint,
		amount_b_min: BigUint) -> SCResult<()> {

		require!(
			user_address != Address::zero(),
			"Can't transfer to default address 0x0!"
		);
		let expected_token_a_name = self.liquidity_pool().get_token_a_name();
		let expected_token_b_name = self.liquidity_pool().get_token_b_name();

		require!(actual_token_a_name == expected_token_a_name, "Wrong token a identifier");
		require!(actual_token_b_name == expected_token_b_name, "Wrong token b identifier");

		let mut balance_a = self.liquidity_pool().get_pair_reserve(&expected_token_a_name);
		let mut balance_b = self.liquidity_pool().get_pair_reserve(&expected_token_b_name);
		let total_supply = self.supply().get_total_supply();

		let amount_a = (liquidity.clone() * balance_a.clone()) / total_supply.clone();
		let amount_b = (liquidity.clone() * balance_b.clone()) / total_supply;

		require!(&amount_a > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_b > &0, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_a >= &amount_a_min, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");
		require!(&amount_b >= &amount_b_min, "Pair: INSUFFICIENT_LIQUIDITY_BURNED");

		
		self.supply()._burn(&user_address, &liquidity);

		self.send().direct_esdt_via_transf_exec(&user_address, expected_token_a_name.as_slice(), &amount_a, &[]);
		self.send().direct_esdt_via_transf_exec(&user_address, expected_token_b_name.as_slice(), &amount_b, &[]);

		balance_a -= amount_a;
		balance_b -= amount_b;

		self.liquidity_pool().set_pair_reserve(&expected_token_a_name, &balance_a);
		self.liquidity_pool().set_pair_reserve(&expected_token_b_name, &balance_b);

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
}