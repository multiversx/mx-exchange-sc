#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();



pub mod liquidity_pool;
pub use crate::liquidity_pool::*;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
	Inactive,
	Active
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct StakeAttributes<BigUint: BigUintApi> {
	lp_token_id: TokenIdentifier,
	total_lp_tokens: BigUint,
	total_initial_worth: BigUint,
	total_amount_liquidity: BigUint
}

#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
	fn get_tokens_for_given_position(&self, amount: BigUint) 
		-> ContractCall<BigUint, ((TokenIdentifier, BigUint), (TokenIdentifier, BigUint))>;
}

#[elrond_wasm_derive::contract(StakingImpl)]
pub trait Staking {

	#[module(LiquidityPoolModuleImpl)]
	fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

	#[init]
	fn init(&self, wegld_token_identifier: TokenIdentifier, router_address: Address) {
		self.wegld_token_identifier().set(&wegld_token_identifier);
		self.liquidity_pool().virtual_token_id().set(&wegld_token_identifier);
		self.router_address().set(&router_address);
		self.state().set(&State::Active);
	}

	#[endpoint]
	fn pause(&self) -> SCResult<()> {
		let caller = self.get_caller();
		let router = self.router_address().get();
		require!(caller == router, "Permission denied");
		self.state().set(&State::Inactive);
		Ok(())
	}

	#[endpoint]
	fn resume(&self) -> SCResult<()> {
		let caller = self.get_caller();
		let router = self.router_address().get();
		require!(caller == router, "Permission denied");
		self.state().set(&State::Active);
		Ok(())
	}

	#[endpoint]
	fn add_pair(&self, address: Address, token: TokenIdentifier) -> SCResult<()> {
		require!(self.state().get() == State::Active, "Not active");
		let caller = self.get_caller();
		let router = self.router_address().get();
		require!(caller == router, "Permission denied");
		self.set_pair_for_lp_token(&token, &address);
		self.set_lp_token_for_pair(&address, &token);
		Ok(())
	}

	#[endpoint]
	fn remove_pair(&self, address: Address, token: TokenIdentifier) -> SCResult<()> {
		require!(self.state().get() == State::Active, "Not active");
		let caller = self.get_caller();
		let router = self.router_address().get();
		require!(caller == router, "Permission denied");
		self.clear_pair_for_lp_token(&token, &address);
		self.clear_lp_token_for_pair(&address, &token);
		Ok(())
	}

	#[payable("*")]
	#[endpoint]
	fn stake(
		&self,
		#[payment_token] lp_token: TokenIdentifier,
		#[payment] amount: BigUint,
	) -> SCResult<()> {

		require!(self.state().get() == State::Active, "Not active");
		let pair = self.get_pair_for_lp_token(&lp_token);
		require!(pair != Address::zero(), "Unknown lp token");

		let one_third_gas = self.get_gas_left() / 3;
		let equivalent = contract_call!(self, pair.clone(), PairContractProxy)
			.get_tokens_for_given_position(amount.clone())
			.execute_on_dest_context(one_third_gas, self.send());

		let wegld_amount: BigUint;
		if equivalent.0.0 == self.wegld_token_identifier().get() {
			wegld_amount = equivalent.0.1;
		}
		else if equivalent.1.0 == self.wegld_token_identifier().get() {
			wegld_amount = equivalent.1.1;
		}
		else {
			return sc_error!("Invalid lp token provider");
		}
		require!(wegld_amount > BigUint::zero(), "Cannot stake with amount of 0");

		let liquidity = sc_try!(self.liquidity_pool().add_liquidity(wegld_amount.clone()));
		let attributes = StakeAttributes::<BigUint>{
			lp_token_id: lp_token.clone(),
			total_lp_tokens: amount.clone(),
			total_initial_worth: wegld_amount.clone(),
			total_amount_liquidity: liquidity.clone()
		};

		self.nft_create(&liquidity, &attributes);
		let sft_id = self.sft_staking_token_identifier().get();
		let nonce = self.get_current_esdt_nft_nonce(&self.get_sc_address(), sft_id.as_esdt_identifier());

		self.send().direct_esdt_nft_via_transfer_exec(
			&self.get_caller(),
			sft_id.as_esdt_identifier(),
			nonce,
			&liquidity,
			&[],
		);

		Ok(())
	}

	#[payable("*")]
	#[endpoint(unstake)]
	fn unstake(&self) -> SCResult<()> {

		require!(self.state().get() == State::Active, "Not active");
		let (liquidity, sft_id) = self.call_value().payment_token_pair();
		let sft_nonce = self.call_value().esdt_token_nonce();

		let required_sft_id = self.sft_staking_token_identifier().get();
		require!(sft_id == required_sft_id, "Unknown staking token");

		let nft_info = self.get_esdt_token_data(
			&self.get_sc_address(),
			sft_id.as_esdt_identifier(),
			sft_nonce,
		);

		let attributes: StakeAttributes::<BigUint>;
		match StakeAttributes::<BigUint>::top_decode(nft_info.attributes.clone().as_slice()) {
			Result::Ok(decoded_obj) => {
				attributes = decoded_obj;
			}
			Result::Err(_) => {
				return sc_error!("Decoding error");
			}
		}

		let pair = self.get_pair_for_lp_token(&attributes.lp_token_id);
		require!(pair != Address::zero(), "Unknown lp token");

		let initial_worth = attributes.total_initial_worth.clone() * liquidity.clone() / 
			attributes.total_amount_liquidity.clone();
		require!(initial_worth > 0, "Cannot unstake with intial_worth == 0");
		let lp_tokens = attributes.total_lp_tokens.clone() * liquidity.clone() / 
			attributes.total_amount_liquidity.clone();
		require!(lp_tokens > 0, "Cannot unstake with lp_tokens == 0");

		let reward = sc_try!(self.liquidity_pool().remove_liquidity(liquidity.clone(), initial_worth.clone()));
		if reward != BigUint::zero() {
			//Rewards should always be a part of the actual reserves of wegld.
			//They should never be part of the virtual reserves of wegld.
			sc_try!(self.validate_existing_esdt_tokens(
				&reward,
				&self.wegld_token_identifier().get(),
			));

			self.send().direct_esdt_via_transf_exec(
				&self.get_caller(),
				self.wegld_token_identifier().get().as_esdt_identifier(),
				&reward,
				&[]
			);
		}

		let mut unstake_amount = self.get_unstake_amount(&self.get_caller(), &attributes.lp_token_id);
		unstake_amount += lp_tokens;
		self.set_unstake_amount(&self.get_caller(), &attributes.lp_token_id, &unstake_amount);
		self.set_unbond_epoch(&self.get_caller(), &attributes.lp_token_id, self.get_block_epoch() + 14400); //10 days

		self.nft_burn(sft_nonce, &liquidity);
		Ok(())
	}

	#[endpoint]
	fn unbond(
		&self,
		lp_token: TokenIdentifier
	) -> SCResult<()> {

		require!(self.state().get() == State::Active, "Not active");
		let caller = self.get_caller();
		require!(!self.is_empty_unstake_amount(&caller, &lp_token), "Don't have anything to unbond");
		let block_epoch = self.get_block_epoch();
		let unbond_epoch = self.get_unbond_epoch(&self.get_caller(), &lp_token);
		require!(block_epoch >= unbond_epoch, "Unbond called too early");

		let unstake_amount = self.get_unstake_amount(&self.get_caller(), &lp_token);
		//Unbonding means that the user should get his lp tokens back.
		//Imperfect calculus should result in less or equal to the correct amount.
		//Imperfect calculus should never result in more lp tokens given back.
		sc_try!(self.validate_existing_esdt_tokens(
			&unstake_amount,
			&lp_token
		));

		self.send().direct_esdt_via_transf_exec(
			&self.get_caller(),
			lp_token.as_esdt_identifier(),
			&unstake_amount,
			&[]
		);

		self.clear_unstake_amount(&caller, &lp_token);
		self.clear_unbond_epoch(&caller, &lp_token);
		Ok(())
	}

	#[payable("EGLD")]
	#[endpoint(sftIssue)]
	fn sft_issue(
		&self,
		#[payment] issue_cost: BigUint,
		token_display_name: BoxedBytes,
		token_ticker: BoxedBytes,
	) -> SCResult<AsyncCall<BigUint>> {

		require!(self.state().get() == State::Active, "Not active");
		only_owner!(self, "Permission denied");
		if !self.sft_staking_token_identifier().is_empty() {
			return sc_error!("Already issued");
		}

		let caller = self.get_caller();
		Ok(ESDTSystemSmartContractProxy::new()
			.issue_semi_fungible(
				issue_cost,
				&token_display_name,
				&token_ticker,
				SemiFungibleTokenProperties {
					can_freeze: true,
					can_wipe: true,
					can_pause: true,
					can_change_owner: true,
					can_upgrade: true,
					can_add_special_roles: true,
				},
			)
			.async_call()
			.with_callback(self.callbacks().sft_issue_callback(&caller))
		)
	}

	#[endpoint(setLocalRoles)]
	fn set_local_roles(
		&self,
		#[var_args] roles: VarArgs<EsdtLocalRole>,
	) -> SCResult<AsyncCall<BigUint>> {

		require!(self.state().get() == State::Active, "Not active");
		only_owner!(self, "Permission denied");
		if self.sft_staking_token_identifier().is_empty() {
			return sc_error!("No staking token issued");
		}

		Ok(ESDTSystemSmartContractProxy::new()
			.set_special_roles(
				&self.get_sc_address(),
				self.sft_staking_token_identifier().get().as_esdt_identifier(),
				roles.as_slice(),
			)
			.async_call()
			.with_callback(self.callbacks().change_roles_callback())
		)
	}

	#[callback]
	fn change_roles_callback(&self, #[call_result] result: AsyncCallResult<()>) {
		match result {
			AsyncCallResult::Ok(()) => {
				self.last_error_message().clear();
			},
			AsyncCallResult::Err(message) => {
				self.last_error_message().set(&message.err_msg);
			},
		}
	}

	fn nft_create(&self, amount: &BigUint, attributes: &StakeAttributes<BigUint>) {
		self.send().esdt_nft_create::<StakeAttributes<BigUint>>(
			self.get_gas_left(),
			self.sft_staking_token_identifier().get().as_esdt_identifier(),
			amount,
			&BoxedBytes::empty(),
			&BigUint::zero(),
			&H256::zero(),
			attributes,
			&[BoxedBytes::empty()],
		);
	}

	fn nft_burn(&self, nonce: u64, amount: &BigUint) {
		self.send().esdt_nft_burn(
			self.get_gas_left(),
			self.sft_staking_token_identifier().get().as_esdt_identifier(),
			nonce,
			amount,
		);
	}

	#[callback]
	fn sft_issue_callback(
		&self,
		caller: &Address,
		#[call_result] result: AsyncCallResult<TokenIdentifier>,
	) {
		match result {
			AsyncCallResult::Ok(token_identifier) => {
				if self.sft_staking_token_identifier().is_empty() {
					self.sft_staking_token_identifier().set(&token_identifier);
				}
			},
			AsyncCallResult::Err(_) => {
				let (returned_tokens, token_identifier) = self.call_value().payment_token_pair();
				if token_identifier.is_egld() && returned_tokens > 0 {
					self.send().direct_egld(caller, &returned_tokens, &[]);
				}
			},
		}
	}

	/// Invariant: should never return error.
	#[view(validateExistingEsdtTokens)]
	fn validate_existing_esdt_tokens(
		&self,
		amount: &BigUint,
		token: &TokenIdentifier
	) -> SCResult<()> {
		let balance = self.get_esdt_balance(
			&self.get_sc_address(),
			token.as_esdt_identifier(),
			0,
		);
		require!(amount < &balance, "Existing funds invariant failed");
		Ok(())
	}

	#[view(calculateRewardsForGivenPosition)]
	fn calculate_rewards_for_given_position(
		&self,
		sft_nonce: u64,
		liquidity: BigUint
	) -> SCResult<BigUint> {

		let sft_info = self.get_esdt_token_data(
			&self.get_sc_address(),
			self.sft_staking_token_identifier().get().as_esdt_identifier(),
			sft_nonce,
		);

		let attributes: StakeAttributes::<BigUint>;
		match StakeAttributes::<BigUint>::top_decode(sft_info.attributes.clone().as_slice()) {
			Result::Ok(decoded_obj) => {
				attributes = decoded_obj;
			}
			Result::Err(_) => {
				return sc_error!("Decoding error");
			}
		}

		let initial_worth = attributes.total_initial_worth.clone() * liquidity.clone() /
			attributes.total_amount_liquidity.clone();
		if initial_worth == BigUint::zero() {
			return Ok(BigUint::zero());
		}

		self.liquidity_pool().calculate_reward(liquidity, initial_worth)
	}

	#[view(getPairForLpToken)]
	#[storage_get("pair_for_lp_token")]
	fn get_pair_for_lp_token(&self, lp_token: &TokenIdentifier) -> Address;

	#[storage_set("pair_for_lp_token")]
	fn set_pair_for_lp_token(&self, lp_token: &TokenIdentifier, pair_address: &Address);

	#[storage_clear("pair_for_lp_token")]
	fn clear_pair_for_lp_token(&self, lp_token: &TokenIdentifier, pair_address: &Address);


	#[view(getLpTokenForPair)]
	#[storage_get("lp_token_for_pair")]
	fn get_lp_token_for_pair(&self, pair_address: &Address) -> TokenIdentifier;

	#[storage_set("lp_token_for_pair")]
	fn set_lp_token_for_pair(&self, pair_address: &Address, token: &TokenIdentifier);

	#[storage_is_empty("lp_token_for_pair")]
	fn is_empty_lp_token_for_pair(&self, pair_address: &Address) -> bool;

	#[storage_clear("lp_token_for_pair")]
	fn clear_lp_token_for_pair(&self, pair_address: &Address, token: &TokenIdentifier);


	#[view(getWegldTokenIdentifier)]
	#[storage_mapper("wegld_token_identifier")]
	fn wegld_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

	#[view(getSftStakingTokenIdentifier)]
	#[storage_mapper("sft_staking_token_identifier")]
	fn sft_staking_token_identifier(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;


	#[view(getUnbondEpoch)]
	#[storage_get("unbond_epoch")]
	fn get_unbond_epoch(&self, address: &Address, token: &TokenIdentifier) -> u64;

	#[storage_set("unbond_epoch")]
	fn set_unbond_epoch(&self, address: &Address, token: &TokenIdentifier, epoch: u64);

	#[storage_clear("unbond_epoch")]
	fn clear_unbond_epoch(&self, address: &Address, token: &TokenIdentifier);


	#[view(getUnstakeAmount)]
	#[storage_get("unstake_amount")]
	fn get_unstake_amount(&self, address: &Address, token: &TokenIdentifier) -> BigUint;

	#[storage_set("unstake_amount")]
	fn set_unstake_amount(&self, address: &Address, token: &TokenIdentifier, amount: &BigUint);

	#[storage_clear("unstake_amount")]
	fn clear_unstake_amount(&self, address: &Address, token: &TokenIdentifier);
	
	#[storage_is_empty("unstake_amount")]
	fn is_empty_unstake_amount(&self, address: &Address, token: &TokenIdentifier) -> bool;


	#[view(getLastErrorMessage)]
	#[storage_mapper("last_error_message")]
	fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

	#[view(getRouterAddress)]
	#[storage_mapper("router_address")]
	fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;

	#[view(getState)]
	#[storage_mapper("state")]
	fn state(&self) -> SingleValueMapper<Self::Storage, State>;
}

