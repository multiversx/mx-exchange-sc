#![no_std]

elrond_wasm::imports!();

use farm_staking::{ClaimRewardsResultType, EnterFarmResultType};

pub mod dual_yield_token;
pub mod lp_farm_token;

#[elrond_wasm::contract]
pub trait FarmStakingProxy:
    dual_yield_token::DualYieldTokenModule
    + lp_farm_token::LpFarmTokenModule
    + token_merge::TokenMergeModule
{
    #[init]
    fn init(
        &self,
        lp_farm_address: ManagedAddress,
        staking_farm_address: ManagedAddress,
        pair_address: ManagedAddress,
        staking_token_id: TokenIdentifier,
        lp_farm_token_id: TokenIdentifier,
        staking_farm_token_id: TokenIdentifier,
    ) -> SCResult<()> {
        require!(
            self.blockchain().is_smart_contract(&lp_farm_address),
            "Invalid LP Farm address"
        );
        require!(
            self.blockchain().is_smart_contract(&staking_farm_address),
            "Invalid Staking Farm address"
        );
        require!(
            self.blockchain().is_smart_contract(&pair_address),
            "Invalid Pair address"
        );
        require!(
            staking_token_id.is_valid_esdt_identifier(),
            "Invalid Staking token ID"
        );
        require!(
            lp_farm_token_id.is_valid_esdt_identifier(),
            "Invalid LP token ID"
        );
        require!(
            staking_farm_token_id.is_valid_esdt_identifier(),
            "Invalid Staking Farm token ID"
        );

        self.staking_farm_address().set(&staking_farm_address);
        self.pair_address().set(&pair_address);
        self.staking_token_id().set(&staking_token_id);
        self.lp_farm_token_id().set(&lp_farm_token_id);
        self.staking_farm_token_id().set(&staking_farm_token_id);

        Ok(())
    }

    #[payable("*")]
    #[endpoint(stakeFarmTokens)]
    fn stake_farm_tokens(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<()> {
        let lp_farm_token_payment: EsdtTokenPayment<Self::Api> =
            payments.get(0).ok_or("empty payments")?;
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let lp_farm_token_id = self.lp_farm_token_id().get();
        require!(
            lp_farm_token_payment.token_identifier == lp_farm_token_id,
            "Invalid first payment"
        );
        self.require_all_payments_dual_yield_tokens(&additional_payments)?;

        let staking_farm_token_id = self.staking_farm_token_id().get();
        let mut staking_farm_tokens = ManagedVec::new();
        for p in &additional_payments {
            let attributes = self.get_dual_yield_token_attributes(p.token_nonce)?;
            staking_farm_tokens.push(EsdtTokenPayment::new(
                staking_farm_token_id.clone(),
                attributes.lp_farm_token_nonce,
                p.amount.clone(),
            ));

            self.burn_dual_yield_tokens(p.token_nonce, &p.amount);
        }

        let lp_tokens_in_farm = self.get_lp_tokens_in_farm_position(
            lp_farm_token_payment.token_nonce,
            &lp_farm_token_payment.amount,
        )?;
        let staking_token_amount = self.get_lp_tokens_value_in_staking_token(&lp_tokens_in_farm);
        let staking_farm_address = self.staking_farm_address().get();
        let received_staking_farm_token: EnterFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .stake_farm_through_proxy(staking_farm_tokens, staking_token_amount)
            .execute_on_dest_context();

        let caller = self.blockchain().get_caller();
        self.create_and_send_dual_yield_tokens(
            &caller,
            &received_staking_farm_token.amount,
            lp_farm_token_payment.token_nonce,
            lp_farm_token_payment.amount,
            received_staking_farm_token.token_nonce,
            received_staking_farm_token.amount.clone(),
        );

        Ok(())
    }

    #[payable("*")]
    #[endpoint(claimRewardsFromFarms)]
    fn claim_rewards_from_farms(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<()> {
        self.require_all_payments_dual_yield_tokens(&payments)?;

        let mut lp_farm_tokens = ManagedVec::new();
        let mut staking_farm_tokens = ManagedVec::new();
        let mut new_staking_farm_values = ManagedVec::new();

        let lp_farm_token_id = self.lp_farm_token_id().get();
        let staking_farm_token_id = self.staking_farm_token_id().get();

        for p in &payments {
            let attributes = self.get_dual_yield_token_attributes(p.token_nonce)?;
            let staking_farm_token_amount =
                self.get_staking_farm_token_amount_equivalent(&attributes, &p.amount);

            staking_farm_tokens.push(EsdtTokenPayment::new(
                staking_farm_token_id.clone(),
                attributes.staking_farm_token_nonce,
                staking_farm_token_amount,
            ));

            let lp_farm_token_amount =
                self.get_lp_farm_token_amount_equivalent(&attributes, &p.amount);
            let lp_tokens_in_position = self.get_lp_tokens_in_farm_position(
                attributes.lp_farm_token_nonce,
                &attributes.lp_farm_token_amount,
            )?;
            let new_staking_farm_value =
                self.get_lp_tokens_value_in_staking_token(&lp_tokens_in_position);

            lp_farm_tokens.push(EsdtTokenPayment::new(
                lp_farm_token_id.clone(),
                attributes.lp_farm_token_nonce,
                lp_farm_token_amount,
            ));
            new_staking_farm_values.push(new_staking_farm_value);
        }

        let lp_farm_address = self.lp_farm_address().get();
        let lp_farm_result: ClaimRewardsResultType<Self::Api> = self
            .lp_farm_proxy_obj(lp_farm_address)
            .claim_rewards(OptionalArg::None)
            .with_multi_token_transfer(lp_farm_tokens)
            .execute_on_dest_context();
        let (new_lp_farm_tokens, lp_farm_rewards) = lp_farm_result.into_tuple();

        let staking_farm_address = self.staking_farm_address().get();
        let staking_farm_result: ClaimRewardsResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .claim_rewards_with_new_value(staking_farm_tokens, new_staking_farm_values)
            .execute_on_dest_context();
        let (new_staking_farm_tokens, staking_farm_rewards) = staking_farm_result.into_tuple();

        let caller = self.blockchain().get_caller();
        self.send().direct(
            &caller,
            &lp_farm_rewards.token_identifier,
            lp_farm_rewards.token_nonce,
            &lp_farm_rewards.amount,
            &[],
        );
        self.send().direct(
            &caller,
            &staking_farm_rewards.token_identifier,
            staking_farm_rewards.token_nonce,
            &staking_farm_rewards.amount,
            &[],
        );
        self.create_and_send_dual_yield_tokens(
            &caller,
            &new_staking_farm_tokens.amount,
            new_lp_farm_tokens.token_nonce,
            new_lp_farm_tokens.amount,
            new_staking_farm_tokens.token_nonce,
            new_staking_farm_tokens.amount.clone(),
        );

        Ok(())
    }

    /*
    #[payable("*")]
    #[endpoint(unstakeFarmTokens)]
    fn unstake_farm_tokens(
        &self,
        #[payment_token] payment_token: TokenIdentifier,
        #[payment_nonce] payment_nonce: u64,
        #[payment_amount] payment_amount: BigUint,
    ) -> SCResult<()> {
        self.require_dual_yield_token(&payment_token)?;

        let farm_token_nonce = self.get_farm_token_nonce_from_attributes(payment_nonce)?;

        Ok(())
    }
    */

    // TODO: Call some method in the pair contract
    fn get_lp_tokens_value_in_staking_token(&self, lp_tokens_amount: &BigUint) -> BigUint {
        lp_tokens_amount.clone()
    }

    // proxies

    #[proxy]
    fn staking_farm_proxy_obj(&self, sc_address: ManagedAddress) -> farm_staking::Proxy<Self::Api>;

    #[proxy]
    fn lp_farm_proxy_obj(&self, sc_address: ManagedAddress) -> farm::Proxy<Self::Api>;

    // storage

    #[view(getLpFarmAddress)]
    #[storage_mapper("lpFarmAddress")]
    fn lp_farm_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getStakingFarmAddress)]
    #[storage_mapper("stakingFarmAddress")]
    fn staking_farm_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPairAddress)]
    #[storage_mapper("pairAddress")]
    fn pair_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getStakingTokenId)]
    #[storage_mapper("stakingTokenId")]
    fn staking_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farmTokenId")]
    fn staking_farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
