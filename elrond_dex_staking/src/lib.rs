#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();


#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
	fn get_tokens_for_given_position(&self, amount: BigUint) -> ContractCall<BigUint>;
}

#[elrond_wasm_derive::contract(StakingImpl)]
pub trait Staking {
	#[init]
	fn init(&self, wegld_token_identifier: TokenIdentifier) {
		self.set_wegld_token_identifier(&wegld_token_identifier);
	}

	#[payable("*")]
	#[endpoint(acceptEsdtFees)]
	fn accept_esdt_fee(
		&self,
		#[payment_token] token: TokenIdentifier,
		#[payment] amount: BigUint
	) -> SCResult<()> {
		if token != self.get_wegld_token_identifier() {
			return sc_error!("Unknown fee payment");
		}

		let mut reserve = self.get_wegld_reserve();
		reserve += amount;
		self.set_wegld_reserve(&reserve);
		Ok(())
	}

	#[payable("*")]
	#[endpoint(stakeLpTokens)]
	fn stake_lp_tokens(
		&self,
		#[payment_token] _lp_token: TokenIdentifier,
		#[payment] _amount: BigUint,
	) -> SCResult<AsyncCall<BigUint>> {

		return sc_error!("Not yet implemented!");
	}

	#[payable("*")]
	#[endpoint(unstakeLpTokens)]
	fn unstake_lp_tokens(
		&self,
		#[payment_token] _staking_token: TokenIdentifier,
		#[payment] _amount: BigUint,
	) -> SCResult<AsyncCall<BigUint>> {

		return sc_error!("Not yet implemented!");
	}

	#[view(getPairForLpToken)]
	#[storage_get("pair_for_lp_token")]
	fn get_pair_for_lp_token(&self, lp_token: &TokenIdentifier) -> Address;

	#[storage_set("pair_for_lp_token")]
	fn set_pair_for_lp_token(&self, lp_token: &TokenIdentifier, pair_address: &Address);


	#[view(getLpTokenForPair)]
	#[storage_get("lp_token_for_pair")]
	fn get_lp_token_for_pair(&self, pair_address: &Address) -> TokenIdentifier;

	#[storage_set("lp_token_for_pair")]
	fn set_lp_token_for_pair(&self, pair_address: &Address, token: &TokenIdentifier);

	#[storage_is_empty("lp_token_for_pair")]
	fn is_empty_lp_token_for_pair(&self, pair_address: &Address) -> bool;


	#[view(getWegldTokenIdentifier)]
	#[storage_get("wegld_token_identifier")]
	fn get_wegld_token_identifier(&self) -> TokenIdentifier;

	#[storage_set("wegld_token_identifier")]
	fn set_wegld_token_identifier(&self, token: &TokenIdentifier);


	#[view(getWeGLDReserve)]
	#[storage_get("wegld_reserve")]
	fn get_wegld_reserve(&self) -> BigUint;

	#[storage_set("wegld_reserve")]
	fn set_wegld_reserve(&self, amount: &BigUint);


	#[view(getVirtualWeGLDReserve)]
	#[storage_get("virtual_wegld_reserve")]
	fn get_virtual_wegld_reserve(&self) -> BigUint;

	#[storage_set("virtual_wegld_reserve")]
	fn set_virtual_wegld_reserve(&self, amount: &BigUint);
}
