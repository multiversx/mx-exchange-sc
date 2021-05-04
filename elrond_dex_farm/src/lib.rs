#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Epoch = u64;
type Nonce = u64;
const PENALTY_PRECENT: u64 = 10;
const EXTERN_QUERY_MAX_GAS: u64 = 20000000;
const EXIT_FARM_NO_PENALTY_MIN_EPOCHS: u64 = 3;

pub mod liquidity_pool;
pub use crate::liquidity_pool::*;
pub mod rewards;
pub use crate::rewards::*;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct FarmTokenAttributes<BigUint: BigUintApi> {
    farmed_token_id: TokenIdentifier,
    total_farmed_tokens: BigUint,
    total_initial_worth: BigUint,
    total_amount_liquidity: BigUint,
    entering_epoch: Epoch,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct SftTokenAmountPair<BigUint: BigUintApi> {
    token_id: TokenIdentifier,
    token_nonce: Nonce,
    amount: BigUint,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct TokenAmountPair<BigUint: BigUintApi> {
    token_id: TokenIdentifier,
    amount: BigUint,
}

#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
    fn getTokensForGivenPosition(
        &self,
        amount: BigUint,
    ) -> ContractCall<BigUint, MultiResult2<TokenAmountPair<BigUint>, TokenAmountPair<BigUint>>>;
    fn getEquivalent(
        &self,
        token: TokenIdentifier,
        amount: BigUint,
    ) -> ContractCall<BigUint, BigUint>;
}

#[elrond_wasm_derive::contract(FarmImpl)]
pub trait Farm {
    #[module(LiquidityPoolModuleImpl)]
    fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

    #[module(RewardsModule)]
    fn rewards(&self) -> RewardsModule<T, BigInt, BigUint>;

    #[init]
    fn init(
        &self,
        farming_pool_token_id: TokenIdentifier,
        router_address: Address,
        farm_with_lp_tokens: bool,
    ) {
        self.farming_pool_token_id().set(&farming_pool_token_id);
        self.router_address().set(&router_address);
        self.state().set(&State::Active);
        self.owner().set(&self.blockchain().get_caller());
        self.farm_with_lp_tokens().set(&farm_with_lp_tokens);
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        sc_try!(self.require_permissions());
        self.state().set(&State::Inactive);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        sc_try!(self.require_permissions());
        self.state().set(&State::Active);
        Ok(())
    }

    #[endpoint(addTrustedPairAsOracle)]
    fn add_oracle_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
        address: Address,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        sc_try!(self.require_permissions());
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(
            self.oracle_pair(&first_token, &second_token).is_empty(),
            "Pair already exists as oracle for given tokens"
        );
        require!(
            self.oracle_pair(&second_token, &first_token).is_empty(),
            "Pair already exists as oracle for given tokens"
        );
        self.oracle_pair(&first_token, &second_token).set(&address);
        self.oracle_pair(&second_token, &first_token).set(&address);
        Ok(())
    }

    #[endpoint(removeTrustedPairAsOracle)]
    fn remove_oracle_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
        address: Address,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        sc_try!(self.require_permissions());
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(
            !self.oracle_pair(&first_token, &second_token).is_empty(),
            "Pair doesn't exists as oracle for given tokens"
        );
        require!(
            !self.oracle_pair(&second_token, &first_token).is_empty(),
            "Pair doesn't exists as oracle for given tokens"
        );
        require!(
            self.oracle_pair(&second_token, &first_token).get() == address,
            "Pair oracle has diferent address"
        );
        require!(
            self.oracle_pair(&first_token, &second_token).get() == address,
            "Pair oracle has diferent address"
        );
        self.oracle_pair(&first_token, &second_token).clear();
        self.oracle_pair(&second_token, &first_token).clear();
        Ok(())
    }

    #[endpoint(addAcceptedPairAddressAndLpToken)]
    fn add_accepted_pair(&self, address: Address, token: TokenIdentifier) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        sc_try!(self.require_permissions());
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(address != Address::zero(), "Zero Address");
        require!(token.is_esdt(), "Not an ESDT token");
        require!(
            !self
                .pair_address_for_accepted_lp_token()
                .contains_key(&token),
            "Pair address already exists for LP token"
        );
        require!(
            self.farming_pool_token_id().get() != token,
            "Farming pool token cannot be an accepted LP token"
        );
        self.pair_address_for_accepted_lp_token()
            .insert(token, address);
        Ok(())
    }

    #[endpoint(removeAcceptedPairAddressAndLpToken)]
    fn remove_accepted_pair(&self, address: Address, token: TokenIdentifier) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        sc_try!(self.require_permissions());
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(address != Address::zero(), "Zero Address");
        require!(token.is_esdt(), "Not an ESDT token");
        require!(
            self.pair_address_for_accepted_lp_token()
                .contains_key(&token),
            "No Pair Address for given LP token"
        );
        require!(
            self.pair_address_for_accepted_lp_token()
                .get(&token)
                .unwrap()
                == address,
            "Address does not match Lp token equivalent"
        );
        self.pair_address_for_accepted_lp_token().remove(&token);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(enterFarm)]
    fn enter_farm(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let farm_contribution = sc_try!(self.get_farm_contribution(&token_in, &amount));
        require!(
            farm_contribution > BigUint::zero(),
            "Cannot farm with amount of 0"
        );

        let farming_pool_token_id = self.farming_pool_token_id().get();
        let liquidity = sc_try!(self.liquidity_pool().add_liquidity(
            farm_contribution.clone(),
            farming_pool_token_id,
            token_in.clone()
        ));
        let farm_attributes = FarmTokenAttributes::<BigUint> {
            farmed_token_id: token_in,
            total_farmed_tokens: amount,
            total_initial_worth: farm_contribution,
            total_amount_liquidity: liquidity.clone(),
            entering_epoch: self.blockchain().get_block_epoch(),
        };

        // This 1 is necessary to get_esdt_token_data needed for calculateRewardsForGivenPosition
        let farm_tokens_to_create = liquidity.clone() + BigUint::from(1u64);
        let farm_token_id = self.farm_token_id().get();
        self.create_farm_tokens(&farm_token_id, &farm_tokens_to_create, &farm_attributes);
        let farm_token_nonce = self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            farm_token_id.as_esdt_identifier(),
        );

        let _ = self.send().direct_esdt_nft_via_transfer_exec(
            &self.blockchain().get_caller(),
            farm_token_id.as_esdt_identifier(),
            farm_token_nonce,
            &liquidity,
            &[],
        );

        Ok(())
    }

    #[payable("*")]
    #[endpoint(exitFarm)]
    fn exit_farm(&self) -> SCResult<()> {
        //require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let (liquidity, payment_token_id) = self.call_value().payment_token_pair();
        let token_nonce = self.call_value().esdt_token_nonce();
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        let farm_attributes =
            sc_try!(self.get_farm_attributes(payment_token_id.clone(), token_nonce));
        let initial_worth = farm_attributes.total_initial_worth.clone() * liquidity.clone()
            / farm_attributes.total_amount_liquidity.clone();
        require!(initial_worth > 0, "Cannot unfarm with 0 intial_worth");
        let farmed_token_amount = farm_attributes.total_farmed_tokens.clone() * liquidity.clone()
            / farm_attributes.total_amount_liquidity.clone();
        require!(farmed_token_amount > 0, "Cannot unfarm with 0 farmed_token");

        let farming_pool_token_id = self.farming_pool_token_id().get();
        let reward = sc_try!(self.liquidity_pool().remove_liquidity(
            liquidity.clone(),
            initial_worth,
            farming_pool_token_id.clone(),
            farm_attributes.farmed_token_id.clone(),
        ));
        self.burn(&payment_token_id, token_nonce, &liquidity);

        let caller = self.blockchain().get_caller();
        self.rewards().mint_rewards(&farming_pool_token_id);
        self.send_reward_and_farmed_tokens(
            reward,
            farming_pool_token_id,
            farmed_token_amount,
            farm_attributes.farmed_token_id,
            caller,
            farm_attributes.entering_epoch,
        );

        Ok(())
    }

    fn send_reward_and_farmed_tokens(
        &self,
        reward_amount: BigUint,
        reward_token: TokenIdentifier,
        farmed_amount: BigUint,
        farmed_token: TokenIdentifier,
        address: Address,
        entering_epoch: u64,
    ) {
        if reward_token == farmed_token {
            let send_total = farmed_amount + reward_amount;
            self.send_tokens_and_burn_penalty(farmed_token, send_total, address, entering_epoch);
        } else {
            self.send_tokens_and_burn_penalty(
                reward_token,
                reward_amount,
                address.clone(),
                entering_epoch,
            );
            self.send_tokens_and_burn_penalty(farmed_token, farmed_amount, address, entering_epoch);
        }
    }

    fn send_tokens_and_burn_penalty(
        &self,
        token: TokenIdentifier,
        amount: BigUint,
        address: Address,
        entering_epoch: u64,
    ) {
        if amount > 0 {
            if self.should_apply_penalty(entering_epoch) {
                let penalty_amount = self.get_penalty_amount(amount.clone());
                if penalty_amount > 0 {
                    self.burn_token(&token, &penalty_amount);
                }
                let to_send = amount - penalty_amount;
                if to_send > 0 {
                    self.send_tokens(&token, &to_send, &address);
                }
            } else {
                self.send_tokens(&token, &amount, &address);
            }
        }
    }

    #[inline]
    fn burn_token(&self, token: &TokenIdentifier, amount: &BigUint) {
        self.send().esdt_local_burn(
            self.blockchain().get_gas_left(),
            token.as_esdt_identifier(),
            &amount,
        );
    }

    #[inline]
    fn send_tokens(&self, token: &TokenIdentifier, amount: &BigUint, destination: &Address) {
        let _ = self.send().direct_esdt_via_transf_exec(
            destination,
            token.as_esdt_identifier(),
            amount,
            &[],
        );
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        token_nonce: u64,
        liquidity: BigUint,
    ) -> SCResult<BigUint> {
        let token_id = self.farm_token_id().get();
        let token_current_nonce = self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            token_id.as_esdt_identifier(),
        );
        require!(token_nonce <= token_current_nonce, "Invalid nonce");

        let attributes = sc_try!(self.get_farm_attributes(token_id, token_nonce));
        let initial_worth = attributes.total_initial_worth.clone() * liquidity.clone()
            / attributes.total_amount_liquidity;
        if initial_worth == 0 {
            return Ok(initial_worth);
        }

        let reward = sc_try!(self.rewards().calculate_reward_for_given_liquidity(
            liquidity,
            initial_worth,
            self.farming_pool_token_id().get(),
        ));

        if self.should_apply_penalty(attributes.entering_epoch) {
            Ok(reward.clone() - self.get_penalty_amount(reward))
        } else {
            Ok(reward)
        }
    }

    #[payable("EGLD")]
    #[endpoint(issueFarmToken)]
    fn issue_farm_token(
        &self,
        #[payment] issue_cost: BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        sc_try!(self.require_permissions());
        require!(self.farm_token_id().is_empty(), "Already issued");

        Ok(self.issue_token(issue_cost, token_display_name, token_ticker))
    }

    fn issue_token(
        &self,
        issue_cost: BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
    ) -> AsyncCall<BigUint> {
        ESDTSystemSmartContractProxy::new()
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
            .with_callback(
                self.callbacks()
                    .issue_callback(&self.blockchain().get_caller()),
            )
    }

    #[callback]
    fn issue_callback(
        &self,
        caller: &Address,
        #[call_result] result: AsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            AsyncCallResult::Ok(token_id) => {
                if self.farm_token_id().is_empty() {
                    self.farm_token_id().set(&token_id);
                }
            }
            AsyncCallResult::Err(_) => {
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        sc_try!(self.require_permissions());
        require!(!self.farm_token_id().is_empty(), "No farm token issued");

        let token = self.farm_token_id().get();
        Ok(self.set_local_roles(token))
    }

    fn set_local_roles(&self, token: TokenIdentifier) -> AsyncCall<BigUint> {
        ESDTSystemSmartContractProxy::new()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                token.as_esdt_identifier(),
                &[
                    EsdtLocalRole::NftCreate,
                    EsdtLocalRole::NftAddQuantity,
                    EsdtLocalRole::NftBurn,
                ],
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: AsyncCallResult<()>) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    fn get_farm_attributes(
        &self,
        token_id: TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<FarmTokenAttributes<BigUint>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id.as_esdt_identifier(),
            token_nonce,
        );

        let farm_attributes = token_info.decode_attributes::<FarmTokenAttributes<BigUint>>();
        match farm_attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn create_farm_tokens(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &FarmTokenAttributes<BigUint>,
    ) {
        self.send().esdt_nft_create::<FarmTokenAttributes<BigUint>>(
            self.blockchain().get_gas_left(),
            token_id.as_esdt_identifier(),
            amount,
            &BoxedBytes::empty(),
            &BigUint::zero(),
            &H256::zero(),
            attributes,
            &[BoxedBytes::empty()],
        );
    }

    fn burn(&self, token: &TokenIdentifier, nonce: u64, amount: &BigUint) {
        self.send().esdt_nft_burn(
            self.blockchain().get_gas_left(),
            token.as_esdt_identifier(),
            nonce,
            amount,
        );
    }

    fn require_permissions(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.owner().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, "Permission denied");
        Ok(())
    }

    fn is_accepted_token(
        &self,
        farming_pool_token_id: &TokenIdentifier,
        token_id: &TokenIdentifier,
    ) -> bool {
        if self.farm_with_lp_tokens().get() {
            self.pair_address_for_accepted_lp_token()
                .contains_key(token_id)
        } else {
            farming_pool_token_id == token_id
        }
    }

    #[view(simulateEnterFarm)]
    fn simulate_enter_farm(
        &self,
        token_in: TokenIdentifier,
        amount_in: BigUint,
    ) -> SCResult<SftTokenAmountPair<BigUint>> {
        let farm_contribution = sc_try!(self.get_farm_contribution(&token_in, &amount_in));
        let farming_pool_token_id = self.farming_pool_token_id().get();

        let liquidity = self.liquidity_pool().calculate_liquidity(
            &farm_contribution,
            &farming_pool_token_id,
            &token_in,
        );
        let farm_token_id = self.farm_token_id().get();
        let farming_pool_token_nonce = self.blockchain().get_current_esdt_nft_nonce(
            &self.blockchain().get_sc_address(),
            &farm_token_id.as_esdt_identifier(),
        );
        Ok(SftTokenAmountPair {
            token_id: farm_token_id,
            token_nonce: farming_pool_token_nonce + 1,
            amount: liquidity,
        })
    }

    #[view(simulateExitFarm)]
    fn simulate_exit_farm(
        &self,
        token_id: TokenIdentifier,
        token_nonce: Nonce,
        amount: BigUint,
    ) -> SCResult<MultiResult2<TokenAmountPair<BigUint>, TokenAmountPair<BigUint>>> {
        let farm_token_id = self.farm_token_id().get();
        require!(token_id == farm_token_id, "Wrong input token");

        let farm_attributes = sc_try!(self.get_farm_attributes(token_id, token_nonce));
        let initial_worth = farm_attributes.total_initial_worth.clone() * amount.clone()
            / farm_attributes.total_amount_liquidity.clone();
        require!(initial_worth > 0, "Cannot unfarm with 0 intial_worth");
        let farmed_token_amount = farm_attributes.total_farmed_tokens.clone() * amount.clone()
            / farm_attributes.total_amount_liquidity.clone();
        let reward = sc_try!(self.calculate_rewards_for_given_position(token_nonce, amount));
        let farming_pool_token_id = self.farming_pool_token_id().get();

        Ok((
            TokenAmountPair {
                token_id: farm_attributes.farmed_token_id,
                amount: farmed_token_amount,
            },
            TokenAmountPair {
                token_id: farming_pool_token_id,
                amount: reward,
            },
        )
            .into())
    }

    #[view(getFarmContribution)]
    fn get_farm_contribution(
        &self,
        token_in: &TokenIdentifier,
        amount_in: &BigUint,
    ) -> SCResult<BigUint> {
        require!(amount_in > &0, "Zero amount in");
        let farming_pool_token_id = self.farming_pool_token_id().get();
        require!(
            self.is_accepted_token(&farming_pool_token_id, &token_in),
            "Token is not accepted for farming"
        );
        if &farming_pool_token_id == token_in {
            return Ok(amount_in.clone());
        }

        let pair = self
            .pair_address_for_accepted_lp_token()
            .get(&token_in)
            .unwrap();
        let gas_limit = core::cmp::min(self.blockchain().get_gas_left(), EXTERN_QUERY_MAX_GAS);
        let equivalent = contract_call!(self, pair, PairContractProxy)
            .getTokensForGivenPosition(amount_in.clone())
            .execute_on_dest_context(gas_limit, self.send());

        let token_amount_pair_tuple = equivalent.0;
        let first_token_amount_pair = token_amount_pair_tuple.0;
        let second_token_amount_pair = token_amount_pair_tuple.1;

        if first_token_amount_pair.token_id == farming_pool_token_id {
            return Ok(first_token_amount_pair.amount);
        } else if second_token_amount_pair.token_id == farming_pool_token_id {
            return Ok(second_token_amount_pair.amount);
        }

        let zero = BigUint::zero();
        let first_query_amount = if !self
            .oracle_pair(&first_token_amount_pair.token_id, &farming_pool_token_id)
            .is_empty()
            && first_token_amount_pair.amount != 0
        {
            self.ask_for_equivalent(
                &first_token_amount_pair.token_id,
                &first_token_amount_pair.amount,
                &farming_pool_token_id,
            )
        } else {
            zero.clone()
        };

        let second_query_amount = if !self
            .oracle_pair(&second_token_amount_pair.token_id, &farming_pool_token_id)
            .is_empty()
            && second_token_amount_pair.amount != 0
        {
            self.ask_for_equivalent(
                &second_token_amount_pair.token_id,
                &second_token_amount_pair.amount,
                &farming_pool_token_id,
            )
        } else {
            zero
        };

        Ok(core::cmp::max(first_query_amount, second_query_amount))
    }

    fn ask_for_equivalent(
        &self,
        token_to_ask: &TokenIdentifier,
        token_to_ask_amount: &BigUint,
        farming_pool_token_id: &TokenIdentifier,
    ) -> BigUint {
        let oracle_pair_to_ask = self.oracle_pair(token_to_ask, farming_pool_token_id).get();
        let gas_limit = core::cmp::min(self.blockchain().get_gas_left(), EXTERN_QUERY_MAX_GAS);
        contract_call!(self, oracle_pair_to_ask, PairContractProxy)
            .getEquivalent(token_to_ask.clone(), token_to_ask_amount.clone())
            .execute_on_dest_context(gas_limit, self.send())
    }

    #[inline]
    fn should_apply_penalty(&self, entering_epoch: Epoch) -> bool {
        entering_epoch + EXIT_FARM_NO_PENALTY_MIN_EPOCHS >= self.blockchain().get_block_epoch()
    }

    #[inline]
    fn get_penalty_amount(&self, amount: BigUint) -> BigUint {
        amount * BigUint::from(PENALTY_PRECENT) / BigUint::from(100u64)
    }

    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active
    }

    #[view(getFarmingPoolTokenIdAndAmounts)]
    fn get_farming_pool_token_id_and_amounts(
        &self,
    ) -> SCResult<(TokenIdentifier, (BigUint, BigUint))> {
        require!(!self.farming_pool_token_id().is_empty(), "Not issued");
        let token = self.farming_pool_token_id().get();
        let vamount = self.liquidity_pool().virtual_reserves().get();
        let amount = self.blockchain().get_esdt_balance(
            &self.blockchain().get_sc_address(),
            token.as_esdt_identifier(),
            0,
        );
        Ok((token, (vamount, amount)))
    }

    #[view(getAllAcceptedTokens)]
    fn get_all_accepted_tokens(&self) -> MultiResultVec<TokenIdentifier> {
        if self.farm_with_lp_tokens().get() {
            self.pair_address_for_accepted_lp_token().keys().collect()
        } else {
            let mut result = MultiResultVec::<TokenIdentifier>::new();
            result.push(self.farming_pool_token_id().get());
            result
        }
    }

    #[storage_mapper("pair_address_for_accepted_lp_token")]
    fn pair_address_for_accepted_lp_token(
        &self,
    ) -> MapMapper<Self::Storage, TokenIdentifier, Address>;

    #[storage_mapper("oracle_pair")]
    fn oracle_pair(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
    ) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getFarmingPoolTokenId)]
    #[storage_mapper("farming_pool_token_id")]
    fn farming_pool_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farm_token_id")]
    fn farm_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getRouterAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, State>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[storage_mapper("farm_with_lp_tokens")]
    fn farm_with_lp_tokens(&self) -> SingleValueMapper<Self::Storage, bool>;
}
