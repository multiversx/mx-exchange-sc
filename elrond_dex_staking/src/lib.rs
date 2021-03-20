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
		self.wegld_token_identifier().set(&wegld_token_identifier);
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
	#[storage_mapper("wegld_token_identifier")]
	fn wegld_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

	#[view(getVirtualWeGLDReserve)]
	#[storage_mapper("virtual_wegld_reserve")]
	fn virtual_wegld_reserve(&self) -> SingleValueMapper<Self::Storage, BigUint>;


	#[view(getSftStakingTokenIdentifier)]
	#[storage_mapper("sft_staking_token_identifier")]
	fn sft_staking_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
