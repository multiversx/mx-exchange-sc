#![no_std]
#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Epoch = u64;
type Nonce = u64;
const BURN_TOKENS_GAS_LIMIT: u64 = 5000000;
const EXIT_FARM_MIN_EPOCHS: u64 = 3;

mod config;
mod liquidity_pool;
mod rewards;

use config::*;
use dex_common::*;

type EnterFarmResultType<BigUint> = SftTokenAmountPair<BigUint>;
type ClaimRewardsResultType<BigUint> =
    MultiResult2<SftTokenAmountPair<BigUint>, TokenAmountPair<BigUint>>;
type ExitFarmResultType<BigUint> = MultiResult2<TokenAmountPair<BigUint>, TokenAmountPair<BigUint>>;

#[derive(TopEncode, TopDecode, TypeAbi)]
pub struct FarmTokenAttributes<BigUint: BigUintApi> {
    total_entering_amount: BigUint,
    total_liquidity_amount: BigUint,
    entering_epoch: Epoch,
}

#[elrond_wasm_derive::contract]
pub trait Farm:
    liquidity_pool::LiquidityPoolModule + rewards::RewardsModule + config::ConfigModule
{
    #[init]
    fn init(
        &self,
        router_address: Address,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
    ) {
        self.state().set(&State::Active);
        self.owner().set(&self.blockchain().get_caller());
        self.router_address().set(&router_address);
        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);
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

    #[payable("*")]
    #[endpoint]
    fn enterFarm(
        &self,
        #[payment_token] token_in: TokenIdentifier,
        #[payment] enter_amount: Self::BigUint,
    ) -> SCResult<EnterFarmResultType<Self::BigUint>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let farming_token_id = self.farming_token_id().get();
        require!(token_in == farming_token_id, "Bad input token");
        require!(enter_amount > 0, "Cannot farm with amount of 0");

        let is_first_provider = self.is_first_provider();
        let mut liquidity = self.add_liquidity(&enter_amount)?;
        let attributes = FarmTokenAttributes {
            total_entering_amount: enter_amount,
            total_liquidity_amount: liquidity.clone(),
            entering_epoch: self.blockchain().get_block_epoch(),
        };

        // Do the actual permanent lock of first minimum liquidity
        // only after the token attributes are crafted for the user.
        if is_first_provider {
            liquidity -= Self::BigUint::from(self.minimum_liquidity_farm_amount());
        }

        let caller = self.blockchain().get_caller();
        let farm_token_id = self.farm_token_id().get();
        let new_nonce = self.create_farm_tokens(&liquidity, &farm_token_id, &attributes);
        self.send()
            .transfer_tokens(&farm_token_id, new_nonce, &liquidity, &caller);

        Ok(SftTokenAmountPair {
            token_id: farm_token_id,
            token_nonce: new_nonce,
            amount: liquidity,
        })
    }

    #[payable("*")]
    #[endpoint]
    fn exitFarm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment] liquidity: Self::BigUint,
    ) -> SCResult<ExitFarmResultType<Self::BigUint>> {
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let token_nonce = self.call_value().esdt_token_nonce();
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Bad input token");

        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;
        let enter_amount = &farm_attributes.total_entering_amount * &liquidity
            / farm_attributes.total_liquidity_amount.clone();
        require!(
            farm_attributes.entering_epoch + EXIT_FARM_MIN_EPOCHS
                < self.blockchain().get_block_epoch(),
            "Exit too soon"
        );
        require!(enter_amount > 0, "Cannot exit farm with 0 entering amount");

        // Before removing liquidity, first generate the rewards.
        let reward_token_id = self.reward_token_id().get();
        self.increase_actual_reserves(&self.mint_rewards(&reward_token_id));

        let caller = self.blockchain().get_caller();
        let reward = self.remove_liquidity(&liquidity, &enter_amount)?;
        let farming_token_id = self.farming_token_id().get();
        self.send().burn_tokens(
            &farm_token_id,
            token_nonce,
            &liquidity,
            BURN_TOKENS_GAS_LIMIT,
        );

        self.send()
            .transfer_tokens(&farming_token_id, 0, &enter_amount, &caller);
        self.send()
            .transfer_tokens(&reward_token_id, 0, &reward, &caller);

        Ok((
            TokenAmountPair {
                token_id: farming_token_id,
                amount: enter_amount,
            },
            TokenAmountPair {
                token_id: reward_token_id,
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
        #[payment] liquidity: Self::BigUint,
    ) -> SCResult<ClaimRewardsResultType<Self::BigUint>> {
        require!(self.is_active(), "Not active");
        require!(!self.farm_token_id().is_empty(), "No issued farm token");
        let token_nonce = self.call_value().esdt_token_nonce();
        let farm_token_id = self.farm_token_id().get();
        require!(payment_token_id == farm_token_id, "Unknown farm token");

        // Read the attributes from the received SFT.
        let farm_attributes = self.get_farm_attributes(&payment_token_id, token_nonce)?;
        let entering_amount = &farm_attributes.total_entering_amount * &liquidity
            / farm_attributes.total_liquidity_amount.clone();
        require!(
            entering_amount > 0,
            "Cannot exit farm with 0 entering amount"
        );

        // Before removing liquidity, first generate the rewards.
        let reward_token_id = self.reward_token_id().get();
        self.increase_actual_reserves(&self.mint_rewards(&reward_token_id));

        // Remove liquidity and burn the received SFT.
        let reward = self.remove_liquidity(&liquidity, &entering_amount)?;
        self.send().burn_tokens(
            &payment_token_id,
            token_nonce,
            &liquidity,
            BURN_TOKENS_GAS_LIMIT,
        );

        // Add the liquidity again, create and send new SFT.
        let re_added_liquidity = self.add_liquidity(&entering_amount)?;
        let caller = self.blockchain().get_caller();
        let new_attributes = FarmTokenAttributes {
            total_entering_amount: entering_amount.clone(),
            total_liquidity_amount: re_added_liquidity.clone(),
            entering_epoch: farm_attributes.entering_epoch,
        };
        let new_nonce =
            self.create_farm_tokens(&re_added_liquidity, &farm_token_id, &new_attributes);
        self.send()
            .transfer_tokens(&farm_token_id, new_nonce, &re_added_liquidity, &caller);

        // Send rewards
        self.send()
            .transfer_tokens(&reward_token_id, 0, &reward, &caller);

        Ok((
            SftTokenAmountPair {
                token_id: farm_token_id,
                token_nonce: new_nonce,
                amount: re_added_liquidity,
            },
            TokenAmountPair {
                token_id: reward_token_id,
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
        #[payment] amount: Self::BigUint,
    ) -> SCResult<()> {
        let reward_token_id = self.reward_token_id().get();
        require!(token_in == reward_token_id, "Bad fee token identifier");
        self.increase_actual_reserves(&amount);
        Ok(())
    }

    #[view(calculateRewardsForGivenPosition)]
    fn calculate_rewards_for_given_position(
        &self,
        liquidity: Self::BigUint,
        attributes_raw: BoxedBytes,
    ) -> SCResult<Self::BigUint> {
        require!(liquidity > 0, "Zero liquidity input");
        let farm_token_supply = self.farm_token_supply().get();
        require!(farm_token_supply > liquidity, "Not enough supply");

        let attributes = self.decode_attributes(&attributes_raw)?;
        require!(
            liquidity <= attributes.total_liquidity_amount,
            "Bad arguments"
        );

        let entering_amount =
            &attributes.total_entering_amount * &liquidity / attributes.total_liquidity_amount;
        Ok(self.calculate_reward_for_given_liquidity(
            &liquidity,
            &entering_amount,
            &farm_token_supply,
            &self.virtual_reserves().get(),
            &self.actual_reserves().get(),
        ))
    }

    #[view(decodeAttributes)]
    fn decode_attributes_endpoint(
        &self,
        attributes_raw: BoxedBytes,
    ) -> SCResult<MultiResultVec<BoxedBytes>> {
        let mut result = Vec::new();
        let attributes = self.decode_attributes(&attributes_raw)?;

        result.push(b"total_entering_amount"[..].into());
        result.push(
            attributes
                .total_entering_amount
                .to_bytes_be()
                .as_slice()
                .into(),
        );

        result.push(b"total_liquidity_amount"[..].into());
        result.push(
            attributes
                .total_liquidity_amount
                .to_bytes_be()
                .as_slice()
                .into(),
        );

        result.push(b"entering_epoch"[..].into());
        result.push(attributes.entering_epoch.to_be_bytes()[..].into());

        Ok(result.into())
    }

    #[payable("EGLD")]
    #[endpoint(issueFarmToken)]
    fn issue_farm_token(
        &self,
        #[payment] issue_cost: Self::BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.farm_token_id().is_empty(), "Already issued");

        Ok(self.issue_token(issue_cost, token_display_name, token_ticker))
    }

    fn issue_token(
        &self,
        issue_cost: Self::BigUint,
        token_display_name: BoxedBytes,
        token_ticker: BoxedBytes,
    ) -> AsyncCall<Self::SendApi> {
        ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
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
    fn set_local_roles_farm_token(&self) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(!self.farm_token_id().is_empty(), "No farm token issued");

        let token = self.farm_token_id().get();
        Ok(self.set_local_roles(token))
    }

    fn set_local_roles(&self, token: TokenIdentifier) -> AsyncCall<Self::SendApi> {
        ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
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

    fn decode_attributes(
        &self,
        attributes_raw: &BoxedBytes,
    ) -> SCResult<FarmTokenAttributes<Self::BigUint>> {
        let attributes =
            <FarmTokenAttributes<Self::BigUint>>::top_decode(attributes_raw.as_slice());
        match attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn get_farm_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<FarmTokenAttributes<Self::BigUint>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id.as_esdt_identifier(),
            token_nonce,
        );

        let farm_attributes = token_info.decode_attributes::<FarmTokenAttributes<Self::BigUint>>();
        match farm_attributes {
            Result::Ok(decoded_obj) => Ok(decoded_obj),
            Result::Err(_) => {
                return sc_error!("Decoding error");
            }
        }
    }

    fn create_farm_tokens(
        &self,
        liquidity: &Self::BigUint,
        farm_token_id: &TokenIdentifier,
        attributes: &FarmTokenAttributes<Self::BigUint>,
    ) -> Nonce {
        self.send()
            .esdt_nft_create::<FarmTokenAttributes<Self::BigUint>>(
                self.blockchain().get_gas_left(),
                farm_token_id.as_esdt_identifier(),
                liquidity,
                &BoxedBytes::empty(),
                &Self::BigUint::zero(),
                &H256::zero(),
                attributes,
                &[BoxedBytes::empty()],
            );
        self.increase_nonce()
    }

    fn increase_nonce(&self) -> Nonce {
        let new_nonce = self.farm_token_nonce().get() + 1;
        self.farm_token_nonce().set(&new_nonce);
        new_nonce
    }

    #[view(getFarmTokenId)]
    #[storage_mapper("farm_token_id")]
    fn farm_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("farm_token_nonce")]
    fn farm_token_nonce(&self) -> SingleValueMapper<Self::Storage, Nonce>;
}
