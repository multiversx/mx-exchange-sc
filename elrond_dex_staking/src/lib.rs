#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

const UNBOND_EPOCH_PERIOD: u64 = 10;

pub mod liquidity_pool;
pub use crate::liquidity_pool::*;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum IssueRequestType {
    Stake,
    Unstake,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct StakeAttributes<BigUint: BigUintApi> {
    lp_token_id: TokenIdentifier,
    total_lp_tokens: BigUint,
    total_initial_worth: BigUint,
    total_amount_liquidity: BigUint,
}

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct UnstakeAttributes {
    lp_token_id: TokenIdentifier,
    unbond_epoch: u64,
}

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub struct TokenAmountPair<BigUint: BigUintApi> {
    token_id: TokenIdentifier,
    amount: BigUint,
}

#[elrond_wasm_derive::callable(PairContractProxy)]
pub trait PairContract {
    fn get_tokens_for_given_position(
        &self,
        amount: BigUint,
    ) -> ContractCall<BigUint, MultiResult2<TokenAmountPair<BigUint>, TokenAmountPair<BigUint>>>;
}

#[elrond_wasm_derive::contract(StakingImpl)]
pub trait Staking {
    #[module(LiquidityPoolModuleImpl)]
    fn liquidity_pool(&self) -> LiquidityPoolModuleImpl<T, BigInt, BigUint>;

    #[init]
    fn init(&self, wegld_token_id: TokenIdentifier, router_address: Address) {
        self.wegld_token_id().set(&wegld_token_id);
        self.router_address().set(&router_address);
        self.state().set(&true);
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "Permission denied");
        self.state().set(&false);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "Permission denied");
        self.state().set(&true);
        Ok(())
    }

    #[endpoint]
    fn add_pair(&self, address: Address, token: TokenIdentifier) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "Permission denied");
        self.set_pair_for_lp_token(&token, &address);
        self.set_lp_token_for_pair(&address, &token);
        Ok(())
    }

    #[endpoint]
    fn remove_pair(&self, address: Address, token: TokenIdentifier) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        let caller = self.get_caller();
        let router = self.router_address().get();
        require!(caller == router, "Permission denied");
        self.clear_pair_for_lp_token(&token);
        self.clear_lp_token_for_pair(&address);
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn stake(
        &self,
        #[payment_token] lp_token: TokenIdentifier,
        #[payment] amount: BigUint,
    ) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            !self.is_empty_pair_for_lp_token(&lp_token),
            "Unknown lp token"
        );
        require!(!self.stake_token_id().is_empty(), "No issued unstake token");
        let pair = self.get_pair_for_lp_token(&lp_token);
        require!(pair != Address::zero(), "Unknown lp token");

        let one_third_gas = self.get_gas_left() / 3;
        let equivalent = contract_call!(self, pair, PairContractProxy)
            .get_tokens_for_given_position(amount.clone())
            .execute_on_dest_context(one_third_gas, self.send());

        let wegld_amount: BigUint;
        let wegld_token_id = self.wegld_token_id().get();
        let token_amount_pair_tuple = equivalent.0;
        if token_amount_pair_tuple.0.token_id == wegld_token_id {
            wegld_amount = token_amount_pair_tuple.0.amount;
        } else if token_amount_pair_tuple.1.token_id == wegld_token_id {
            wegld_amount = token_amount_pair_tuple.1.amount;
        } else {
            return sc_error!("Invalid lp token provider");
        }
        require!(
            wegld_amount > BigUint::zero(),
            "Cannot stake with amount of 0"
        );

        let liquidity = sc_try!(self
            .liquidity_pool()
            .add_liquidity(wegld_amount.clone(), wegld_token_id));
        let stake_attributes = StakeAttributes::<BigUint> {
            lp_token_id: lp_token,
            total_lp_tokens: amount,
            total_initial_worth: wegld_amount,
            total_amount_liquidity: liquidity.clone(),
        };

        // This 1 is necessary to get_esdt_token_data needed for calculateRewardsForGivenPosition
        let stake_tokens_to_create = liquidity.clone() + BigUint::from(1u64);
        let stake_token_id = self.stake_token_id().get();
        self.create_stake_tokens(&stake_token_id, &stake_tokens_to_create, &stake_attributes);
        let stake_token_nonce = self.get_current_esdt_nft_nonce(
            &self.get_sc_address(),
            stake_token_id.as_esdt_identifier(),
        );

        self.send().direct_esdt_nft_via_transfer_exec(
            &self.get_caller(),
            stake_token_id.as_esdt_identifier(),
            stake_token_nonce,
            &liquidity,
            &[],
        );

        Ok(())
    }

    #[payable("*")]
    #[endpoint(unstake)]
    fn unstake(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(!self.stake_token_id().is_empty(), "No issued stake token");
        require!(
            !self.unstake_token_id().is_empty(),
            "No issued unstake token"
        );
        let (liquidity, payment_token_id) = self.call_value().payment_token_pair();
        let token_nonce = self.call_value().esdt_token_nonce();
        let stake_token_id = self.stake_token_id().get();
        require!(payment_token_id == stake_token_id, "Unknown stake token");

        let stake_attributes =
            sc_try!(self.get_stake_attributes(payment_token_id.clone(), token_nonce));
        let initial_worth = stake_attributes.total_initial_worth.clone() * liquidity.clone()
            / stake_attributes.total_amount_liquidity.clone();
        require!(initial_worth > 0, "Cannot unstake with 0 intial_worth");
        let lp_tokens = stake_attributes.total_lp_tokens.clone() * liquidity.clone()
            / stake_attributes.total_amount_liquidity.clone();
        require!(lp_tokens > 0, "Cannot unstake with 0 lp_tokens");

        let wegld_token_id = self.wegld_token_id().get();
        let reward = sc_try!(self.liquidity_pool().remove_liquidity(
            liquidity.clone(),
            initial_worth,
            wegld_token_id.clone()
        ));
        if reward != BigUint::zero() {
            self.send().direct_esdt_via_transf_exec(
                &self.get_caller(),
                wegld_token_id.as_esdt_identifier(),
                &reward,
                &[],
            );
        }
        self.burn(&payment_token_id, token_nonce, &liquidity);

        let unstake_attributes = UnstakeAttributes {
            lp_token_id: stake_attributes.lp_token_id,
            unbond_epoch: self.get_block_epoch() + UNBOND_EPOCH_PERIOD,
        };
        let unstake_tokens_to_create = lp_tokens;
        let unstake_token_id = self.unstake_token_id().get();
        self.create_unstake_tokens(
            &unstake_token_id,
            &unstake_tokens_to_create,
            &unstake_attributes,
        );
        let unstake_nonce = self.get_current_esdt_nft_nonce(
            &self.get_sc_address(),
            unstake_token_id.as_esdt_identifier(),
        );

        self.send().direct_esdt_nft_via_transfer_exec(
            &self.get_caller(),
            unstake_token_id.as_esdt_identifier(),
            unstake_nonce,
            &unstake_tokens_to_create,
            &[],
        );

        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn unbond(&self) -> SCResult<()> {
        require!(self.is_active(), "Not active");
        require!(
            !self.unstake_token_id().is_empty(),
            "No issued unstake token"
        );
        let (amount, token_id) = self.call_value().payment_token_pair();
        let token_nonce = self.call_value().esdt_token_nonce();
        let unstake_token_id = self.unstake_token_id().get();
        require!(unstake_token_id == token_id, "Wrong unstake token");

        let unstake_attributes = sc_try!(self.get_unstake_attributes(token_id, token_nonce));
        let block_epoch = self.get_block_epoch();
        let unbond_epoch = unstake_attributes.unbond_epoch;
        require!(block_epoch >= unbond_epoch, "Unbond called too early");

        let unbond_amount = amount;
        let lp_token_unbonded = unstake_attributes.lp_token_id;
        self.send().direct_esdt_via_transf_exec(
            &self.get_caller(),
            lp_token_unbonded.as_esdt_identifier(),
            &unbond_amount,
            &[],
        );

        Ok(())
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        token_nonce: u64,
        liquidity: BigUint,
    ) -> SCResult<BigUint> {
        let token_id = self.stake_token_id().get();
        let token_current_nonce =
            self.get_current_esdt_nft_nonce(&self.get_sc_address(), token_id.as_esdt_identifier());
        require!(token_nonce <= token_current_nonce, "Invalid nonce");

        let attributes = sc_try!(self.get_stake_attributes(token_id, token_nonce));
        let initial_worth = attributes.total_initial_worth.clone() * liquidity.clone()
            / attributes.total_amount_liquidity;
        if initial_worth == BigUint::zero() {
            return Ok(BigUint::zero());
        }

        self.liquidity_pool().calculate_reward(
            liquidity,
            initial_worth,
            self.wegld_token_id().get(),
        )
    }

    #[payable("EGLD")]
    #[endpoint(issueStakeToken)]
    fn issue_stake_token(
        &self,
        #[payment] issue_cost: BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        if !self.stake_token_id().is_empty() {
            return sc_error!("Already issued");
        }

        Ok(self.issue_token(
            issue_cost,
            token_display_name,
            token_ticker,
            IssueRequestType::Stake,
        ))
    }

    #[payable("EGLD")]
    #[endpoint(issueUnstakeToken)]
    fn issue_unstake_token(
        &self,
        #[payment] issue_cost: BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        if !self.unstake_token_id().is_empty() {
            return sc_error!("Already issued");
        }

        Ok(self.issue_token(
            issue_cost,
            token_display_name,
            token_ticker,
            IssueRequestType::Unstake,
        ))
    }

    fn issue_token(
        &self,
        issue_cost: BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
        issue_request: IssueRequestType,
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
                    .issue_callback(&self.get_caller(), issue_request),
            )
    }

    #[callback]
    fn issue_callback(
        &self,
        caller: &Address,
        issue_type: IssueRequestType,
        #[call_result] result: AsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            AsyncCallResult::Ok(token_id) => {
                if issue_type == IssueRequestType::Stake && self.stake_token_id().is_empty() {
                    self.stake_token_id().set(&token_id);
                }
                if issue_type == IssueRequestType::Unstake && self.unstake_token_id().is_empty() {
                    self.unstake_token_id().set(&token_id);
                }
            }
            AsyncCallResult::Err(_) => {
                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[endpoint(setLocalRolesStakeToken)]
    fn set_local_roles_stake_token(
        &self,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        if self.stake_token_id().is_empty() {
            return sc_error!("No stake token issued");
        }

        let token = self.stake_token_id().get();
        Ok(self.set_local_roles(token, roles))
    }

    #[endpoint(setLocalRolesUnstakeToken)]
    fn set_local_roles_unstake_token(
        &self,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall<BigUint>> {
        require!(self.is_active(), "Not active");
        only_owner!(self, "Permission denied");
        if self.unstake_token_id().is_empty() {
            return sc_error!("No stake token issued");
        }

        let token = self.unstake_token_id().get();
        Ok(self.set_local_roles(token, roles))
    }

    fn set_local_roles(
        &self,
        token: TokenIdentifier,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> AsyncCall<BigUint> {
        ESDTSystemSmartContractProxy::new()
            .set_special_roles(
                &self.get_sc_address(),
                token.as_esdt_identifier(),
                roles.as_slice(),
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

    fn get_unstake_attributes(
        &self,
        token_id: TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<UnstakeAttributes> {
        let token_info = self.get_esdt_token_data(
            &self.get_sc_address(),
            token_id.as_esdt_identifier(),
            token_nonce,
        );

        match UnstakeAttributes::top_decode(token_info.attributes.as_slice()) {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn get_stake_attributes(
        &self,
        token_id: TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<StakeAttributes<BigUint>> {
        let token_info = self.get_esdt_token_data(
            &self.get_sc_address(),
            token_id.as_esdt_identifier(),
            token_nonce,
        );

        match StakeAttributes::<BigUint>::top_decode(token_info.attributes.as_slice()) {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn create_stake_tokens(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &StakeAttributes<BigUint>,
    ) {
        self.send().esdt_nft_create::<StakeAttributes<BigUint>>(
            self.get_gas_left(),
            token_id.as_esdt_identifier(),
            amount,
            &BoxedBytes::empty(),
            &BigUint::zero(),
            &H256::zero(),
            attributes,
            &[BoxedBytes::empty()],
        );
    }

    fn create_unstake_tokens(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &UnstakeAttributes,
    ) {
        self.send().esdt_nft_create::<UnstakeAttributes>(
            self.get_gas_left(),
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
            self.get_gas_left(),
            token.as_esdt_identifier(),
            nonce,
            amount,
        );
    }

    fn is_active(&self) -> bool {
        self.state().get()
    }

    #[view(getWegldIdAndAmounts)]
    fn get_wegld_id_and_amounts(&self) -> SCResult<(TokenIdentifier, (BigUint, BigUint))> {
        require!(!self.wegld_token_id().is_empty(), "Not issued");
        let token = self.wegld_token_id().get();
        let vamount = self.liquidity_pool().virtual_reserves().get();
        let amount = self.get_esdt_balance(&self.get_sc_address(), token.as_esdt_identifier(), 0);
        Ok((token, (vamount, amount)))
    }

    #[view(getPairForLpToken)]
    #[storage_get("pair_for_lp_token")]
    fn get_pair_for_lp_token(&self, lp_token: &TokenIdentifier) -> Address;

    #[storage_set("pair_for_lp_token")]
    fn set_pair_for_lp_token(&self, lp_token: &TokenIdentifier, pair_address: &Address);

    #[storage_clear("pair_for_lp_token")]
    fn clear_pair_for_lp_token(&self, lp_token: &TokenIdentifier);

    #[storage_is_empty("pair_for_lp_token")]
    fn is_empty_pair_for_lp_token(&self, lp_token: &TokenIdentifier) -> bool;

    #[view(getLpTokenForPair)]
    #[storage_get("lp_token_for_pair")]
    fn get_lp_token_for_pair(&self, pair_address: &Address) -> TokenIdentifier;

    #[storage_set("lp_token_for_pair")]
    fn set_lp_token_for_pair(&self, pair_address: &Address, token: &TokenIdentifier);

    #[storage_clear("lp_token_for_pair")]
    fn clear_lp_token_for_pair(&self, pair_address: &Address);

    #[view(getWegldTokenId)]
    #[storage_mapper("wegld_token_id")]
    fn wegld_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getStakeTokenId)]
    #[storage_mapper("stake_token_id")]
    fn stake_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getUnstakeTokenId)]
    #[storage_mapper("unstake_token_id")]
    fn unstake_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<Self::Storage, BoxedBytes>;

    #[view(getRouterAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<Self::Storage, Address>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<Self::Storage, bool>;
}
