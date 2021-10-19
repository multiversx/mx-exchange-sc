#![no_std]
#![allow(non_snake_case)]
#![allow(clippy::type_complexity)]

mod liquidity_pool;
mod rewards;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();
use common_structs::{Epoch, FftTokenAmountPair, GenericTokenAmountPair, Nonce};

const PENALTY_PERCENT: u64 = 10;
const EXIT_FARM_NO_PENALTY_MIN_EPOCHS: u64 = 3;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct FarmTokenAttributes<M: ManagedTypeApi> {
    farmed_token_id: TokenIdentifier<M>,
    total_farmed_tokens: BigUint<M>,
    total_initial_worth: BigUint<M>,
    total_amount_liquidity: BigUint<M>,
    entering_epoch: Epoch,
}

/*
    This contract is used at the moment and might not be up to date.
    TODOs:
        -> remove commented lines
        -> remove the +1 when creating farm tokens
        -> change calculate_rewards_for_given_position so it receives token attributes
*/

#[elrond_wasm::contract]
pub trait Farm: liquidity_pool::LiquidityPoolModule + rewards::RewardsModule {
    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> elrond_dex_pair::Proxy<Self::Api>;

    #[init]
    fn init(
        &self,
        farming_pool_token_id: TokenIdentifier,
        router_address: ManagedAddress,
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
        self.require_permissions()?;
        self.state().set(&State::Inactive);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Active);
        Ok(())
    }

    #[endpoint(addTrustedPairAsOracle)]
    fn add_oracle_pair(
        &self,
        first_token: TokenIdentifier,
        second_token: TokenIdentifier,
        address: ManagedAddress,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
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
        address: ManagedAddress,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(
            !self.oracle_pair(&first_token, &second_token).is_empty(),
            "Pair doesn't exist as oracle for given tokens"
        );
        require!(
            !self.oracle_pair(&second_token, &first_token).is_empty(),
            "Pair doesn't exist as oracle for given tokens"
        );
        require!(
            self.oracle_pair(&second_token, &first_token).get() == address,
            "Pair oracle has different address"
        );
        require!(
            self.oracle_pair(&first_token, &second_token).get() == address,
            "Pair oracle has different address"
        );
        self.oracle_pair(&first_token, &second_token).clear();
        self.oracle_pair(&second_token, &first_token).clear();
        Ok(())
    }

    #[endpoint(addAcceptedPairManagedAddressAndLpToken)]
    fn add_accepted_pair(&self, address: ManagedAddress, token: TokenIdentifier) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(
            address != self.types().managed_address_zero(),
            "Zero ManagedAddress"
        );
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

    #[endpoint(removeAcceptedPairManagedAddressAndLpToken)]
    fn remove_accepted_pair(
        &self,
        address: ManagedAddress,
        token: TokenIdentifier,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.farm_with_lp_tokens().get(), "Not an LP token farm");
        require!(
            address != self.types().managed_address_zero(),
            "Zero ManagedAddress"
        );
        require!(token.is_esdt(), "Not an ESDT token");
        require!(
            self.pair_address_for_accepted_lp_token()
                .contains_key(&token),
            "No Pair ManagedAddress for given LP token"
        );
        require!(
            self.pair_address_for_accepted_lp_token()
                .get(&token)
                .unwrap()
                == address,
            "ManagedAddress does not match Lp token equivalent"
        );
        self.pair_address_for_accepted_lp_token().remove(&token);
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn enterFarm(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] amount: BigUint,
    ) -> SCResult<GenericTokenAmountPair<Self::Api>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let farm_contribution = self.get_farm_contribution(&token_in, &amount)?;
        require!(farm_contribution > 0, "Cannot farm with amount of 0");

        let is_first_provider = self.is_first_provider();
        let farming_pool_token_id = self.farming_pool_token_id().get();
        let mut liquidity = self.add_liquidity(
            farm_contribution.clone(),
            farming_pool_token_id,
            token_in.clone(),
        )?;
        let farm_attributes = FarmTokenAttributes::<Self::Api> {
            farmed_token_id: token_in,
            total_farmed_tokens: amount,
            total_initial_worth: farm_contribution,
            total_amount_liquidity: liquidity.clone(),
            entering_epoch: self.blockchain().get_block_epoch(),
        };

        // Do the actual permanent lock of first minimum liquidity
        // only after the token attributes are crafted for the user.
        if is_first_provider {
            liquidity -= BigUint::from(self.minimum_liquidity_farm_amount());
        }

        // This 1 is necessary to get_esdt_token_data needed for calculateRewardsForGivenPosition
        let farm_tokens_to_create = &liquidity + 1u64;
        let farm_token_id = self.farm_token_id().get();
        self.create_farm_tokens(&farm_token_id, &farm_tokens_to_create, &farm_attributes);
        let farm_token_nonce = self.farm_token_nonce().get();

        self.send_tokens(
            &farm_token_id,
            farm_token_nonce,
            &liquidity,
            &self.blockchain().get_caller(),
        );

        Ok(GenericTokenAmountPair {
            token_id: farm_token_id,
            token_nonce: farm_token_nonce,
            amount: liquidity,
        })
    }

    #[payable("*")]
    #[endpoint]
    fn exitFarm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment] liquidity: BigUint,
    ) -> SCResult<MultiResult2<FftTokenAmountPair<Self::Api>, FftTokenAmountPair<Self::Api>>> {
        //require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let token_nonce = self.call_value().esdt_token_nonce();
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        let farm_attributes = self.get_farm_attributes(payment_token_id.clone(), token_nonce)?;
        let initial_worth = &farm_attributes.total_initial_worth * &liquidity
            / farm_attributes.total_amount_liquidity.clone();
        require!(initial_worth > 0, "Cannot unfarm with 0 intial_worth");
        let mut farmed_token_amount = &farm_attributes.total_farmed_tokens * &liquidity
            / farm_attributes.total_amount_liquidity.clone();
        require!(farmed_token_amount > 0, "Cannot unfarm with 0 farmed_token");

        let farming_pool_token_id = self.farming_pool_token_id().get();
        let mut reward = self.remove_liquidity(
            liquidity.clone(),
            initial_worth,
            farming_pool_token_id.clone(),
            farm_attributes.farmed_token_id.clone(),
        )?;
        self.burn_tokens(&payment_token_id, token_nonce, &liquidity);

        let caller = self.blockchain().get_caller();
        self.mint_rewards(&farming_pool_token_id);
        if self.should_apply_penalty(farm_attributes.entering_epoch) {
            let mut penalty_amount = self.get_penalty_amount(reward.clone());
            self.burn_tokens(&farming_pool_token_id, 0, &penalty_amount);
            reward -= penalty_amount;

            penalty_amount = self.get_penalty_amount(farmed_token_amount.clone());
            self.burn_tokens(&farm_attributes.farmed_token_id, 0, &penalty_amount);
            farmed_token_amount -= penalty_amount;
        }

        self.send_tokens(
            &farm_attributes.farmed_token_id,
            0,
            &farmed_token_amount,
            &caller,
        );
        self.send_tokens(&farming_pool_token_id, 0, &reward, &caller);

        Ok((
            FftTokenAmountPair {
                token_id: farm_attributes.farmed_token_id,
                amount: farmed_token_amount,
            },
            FftTokenAmountPair {
                token_id: farming_pool_token_id,
                amount: reward,
            },
        )
            .into())
    }

    #[payable("*")]
    #[endpoint]
    fn claimRewards(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment] liquidity: BigUint,
    ) -> SCResult<MultiResult2<GenericTokenAmountPair<Self::Api>, FftTokenAmountPair<Self::Api>>>
    {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let token_nonce = self.call_value().esdt_token_nonce();
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        // Get info from input tokens and burn them.
        let farm_attributes = self.get_farm_attributes(payment_token_id.clone(), token_nonce)?;
        let initial_worth = &farm_attributes.total_initial_worth * &liquidity
            / farm_attributes.total_amount_liquidity.clone();
        require!(initial_worth > 0, "Cannot unfarm with 0 intial_worth");
        let farmed_token_amount = &farm_attributes.total_farmed_tokens * &liquidity
            / farm_attributes.total_amount_liquidity.clone();
        require!(farmed_token_amount > 0, "Cannot unfarm with 0 farmed_token");
        self.burn_tokens(&payment_token_id, token_nonce, &liquidity);

        // Remove liquidity and send rewards. No penalty.
        let caller = self.blockchain().get_caller();
        let farming_pool_token_id = self.farming_pool_token_id().get();
        let reward = self.remove_liquidity(
            liquidity,
            initial_worth.clone(),
            farming_pool_token_id.clone(),
            farm_attributes.farmed_token_id.clone(),
        )?;
        // Must mint rewards before sending them.
        self.mint_rewards(&farming_pool_token_id);

        // Re-add the lp tokens and their worth into liquidity pool.
        let re_added_liquidity = self.add_liquidity(
            initial_worth.clone(),
            farming_pool_token_id.clone(),
            farm_attributes.farmed_token_id.clone(),
        )?;
        let new_farm_attributes = FarmTokenAttributes::<Self::Api> {
            farmed_token_id: farm_attributes.farmed_token_id,
            total_farmed_tokens: farmed_token_amount,
            total_initial_worth: initial_worth,
            total_amount_liquidity: re_added_liquidity.clone(),
            entering_epoch: farm_attributes.entering_epoch,
        };

        // Create and send the new farm tokens.
        let farm_tokens_to_create = &re_added_liquidity + 1u64;
        self.create_farm_tokens(&farm_token_id, &farm_tokens_to_create, &new_farm_attributes);
        let farm_token_nonce = self.farm_token_nonce().get();

        self.send_tokens(
            &farm_token_id,
            farm_token_nonce,
            &re_added_liquidity,
            &caller,
        );
        self.send_tokens(&farming_pool_token_id, 0, &reward, &caller);

        Ok((
            GenericTokenAmountPair {
                token_id: farm_token_id,
                token_nonce: farm_token_nonce,
                amount: re_added_liquidity,
            },
            FftTokenAmountPair {
                token_id: farming_pool_token_id,
                amount: reward,
            },
        )
            .into())
    }

    #[payable("*")]
    #[endpoint]
    fn acceptFee(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] _amount: BigUint,
    ) -> SCResult<()> {
        let farming_pool_token_id = self.farming_pool_token_id().get();
        require!(
            token_in == farming_pool_token_id,
            "Bad fee token identifier"
        );
        Ok(())
    }

    #[inline]
    fn burn_tokens(&self, token: &TokenIdentifier, nonce: Nonce, amount: &BigUint) {
        if amount > &0 {
            self.send().esdt_local_burn(token, nonce, amount);
        }
    }

    #[inline]
    fn send_tokens(
        &self,
        token: &TokenIdentifier,
        nonce: Nonce,
        amount: &BigUint,
        destination: &ManagedAddress,
    ) {
        if amount > &0 {
            if nonce > 0 {
                let _ = self.send().direct(destination, token, nonce, amount, &[]);
            } else {
                let _ = self.send().direct(destination, token, 0, amount, &[]);
            }
        }
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        token_nonce: u64,
        liquidity: BigUint,
    ) -> SCResult<BigUint> {
        let token_id = self.farm_token_id().get();
        let token_current_nonce = self.farm_token_nonce().get();
        require!(token_nonce <= token_current_nonce, "Invalid nonce");

        let attributes = self.get_farm_attributes(token_id, token_nonce)?;
        let initial_worth =
            &attributes.total_initial_worth * &liquidity / attributes.total_amount_liquidity;
        if initial_worth == 0 {
            return Ok(initial_worth);
        }

        let reward = self.calculate_reward_for_given_liquidity(
            &liquidity,
            &initial_worth,
            &self.farming_pool_token_id().get(),
            &self.total_supply().get(),
            &self.virtual_reserves().get(),
        )?;

        if self.should_apply_penalty(attributes.entering_epoch) {
            Ok(&reward - &self.get_penalty_amount(reward.clone()))
        } else {
            Ok(reward)
        }
    }

    #[payable("EGLD")]
    #[endpoint(issueFarmToken)]
    fn issue_farm_token(
        &self,
        #[payment] issue_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) -> SCResult<AsyncCall> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.farm_token_id().is_empty(), "Already issued");

        Ok(self.issue_token(issue_cost, token_display_name, token_ticker))
    }

    fn issue_token(
        &self,
        issue_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) -> AsyncCall {
        self.send()
            .esdt_system_sc_proxy()
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
        caller: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                if self.farm_token_id().is_empty() {
                    self.farm_token_id().set(&token_id);
                }
            }
            ManagedAsyncCallResult::Err(_) => {
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) -> SCResult<AsyncCall> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(!self.farm_token_id().is_empty(), "No farm token issued");

        let token = self.farm_token_id().get();
        Ok(self.set_local_roles(token))
    }

    fn set_local_roles(&self, token: TokenIdentifier) -> AsyncCall {
        let roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &token,
                (&roles[..]).into_iter().cloned(),
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    fn get_farm_attributes(
        &self,
        token_id: TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<FarmTokenAttributes<Self::Api>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            &token_id,
            token_nonce,
        );

        Ok(self
            .serializer()
            .top_decode_from_managed_buffer::<FarmTokenAttributes<Self::Api>>(
                &token_info.attributes,
            ))
    }

    fn create_farm_tokens(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &FarmTokenAttributes<Self::Api>,
    ) {
        let mut uris = ManagedVec::new(self.type_manager());
        uris.push(self.types().managed_buffer_new());
        self.send()
            .esdt_nft_create::<FarmTokenAttributes<Self::Api>>(
                token_id,
                amount,
                &self.types().managed_buffer_new(),
                &BigUint::zero(),
                &self.types().managed_buffer_new(),
                attributes,
                &uris,
            );

        self.increase_nonce();
    }

    fn increase_nonce(&self) -> Nonce {
        let new_nonce = self.farm_token_nonce().get() + 1;
        self.farm_token_nonce().set(&new_nonce);
        new_nonce
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

    fn get_farm_contribution(
        &self,
        token_in: &TokenIdentifier,
        amount_in: &BigUint,
    ) -> SCResult<BigUint> {
        require!(amount_in > &0, "Zero amount in");
        let farming_pool_token_id = self.farming_pool_token_id().get();
        require!(
            self.is_accepted_token(&farming_pool_token_id, token_in),
            "Token is not accepted for farming"
        );
        if &farming_pool_token_id == token_in {
            return Ok(amount_in.clone());
        }

        let pair = self
            .pair_address_for_accepted_lp_token()
            .get(token_in)
            .unwrap();
        let equivalent = self
            .pair_contract_proxy(pair)
            .get_tokens_for_given_position(amount_in.clone())
            .execute_on_dest_context();

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
        self.pair_contract_proxy(oracle_pair_to_ask)
            .get_equivalent(token_to_ask.clone(), token_to_ask_amount.clone())
            .execute_on_dest_context()
    }

    #[inline]
    fn should_apply_penalty(&self, entering_epoch: Epoch) -> bool {
        entering_epoch + EXIT_FARM_NO_PENALTY_MIN_EPOCHS >= self.blockchain().get_block_epoch()
    }

    #[inline]
    fn get_penalty_amount(&self, amount: BigUint) -> BigUint {
        amount * PENALTY_PERCENT / 100u64
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
        let vamount = self.virtual_reserves().get();
        let amount =
            self.blockchain()
                .get_esdt_balance(&self.blockchain().get_sc_address(), &token, 0);
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
    fn pair_address_for_accepted_lp_token(&self) -> MapMapper<TokenIdentifier, ManagedAddress>;

    #[storage_mapper("oracle_pair")]
    fn oracle_pair(
        &self,
        first_token_id: &TokenIdentifier,
        second_token_id: &TokenIdentifier,
    ) -> SingleValueMapper<ManagedAddress>;

    #[view(getFarmingPoolTokenId)]
    #[storage_mapper("farming_pool_token_id")]
    fn farming_pool_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farm_token_id")]
    fn farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[storage_mapper("farm_token_nonce")]
    fn farm_token_nonce(&self) -> SingleValueMapper<Nonce>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getRouterManagedAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("farm_with_lp_tokens")]
    fn farm_with_lp_tokens(&self) -> SingleValueMapper<bool>;
}
