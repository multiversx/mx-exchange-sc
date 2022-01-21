#![no_std]

elrond_wasm::imports!();

use farm_staking::EnterFarmResultType;

pub mod dual_yield_token;

#[elrond_wasm::contract]
pub trait FarmStakingProxy: dual_yield_token::DualYieldTokenModule {
    #[init]
    fn init(
        &self,
        staking_farm_address: ManagedAddress,
        pair_address: ManagedAddress,
        staking_token_id: TokenIdentifier,
        lp_token_id: TokenIdentifier,
        farm_token_id: TokenIdentifier,
    ) -> SCResult<()> {
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
            lp_token_id.is_valid_esdt_identifier(),
            "Invalid LP token ID"
        );
        require!(
            farm_token_id.is_valid_esdt_identifier(),
            "Invalid Farm token ID"
        );

        self.staking_farm_address().set(&staking_farm_address);
        self.pair_address().set(&pair_address);
        self.staking_token_id().set(&staking_token_id);
        self.lp_token_id().set(&lp_token_id);
        self.farm_token_id().set(&farm_token_id);

        Ok(())
    }

    #[payable("*")]
    #[endpoint(stakeFarmTokens)]
    fn stake_farm_tokens(
        &self,
        #[payment_multi] payments: ManagedVec<EsdtTokenPayment<Self::Api>>,
    ) -> SCResult<()> {
        let first_payment: EsdtTokenPayment<Self::Api> = payments.get(0).ok_or("empty payments")?;
        let additional_payments = payments.slice(1, payments.len()).unwrap_or_default();

        let lp_token_id = self.lp_token_id().get();
        require!(
            first_payment.token_identifier == lp_token_id,
            "Invalid first payment"
        );
        self.require_all_payments_dual_yield_tokens(&additional_payments)?;

        let farm_token_id = self.farm_token_id().get();
        let mut farm_tokens = ManagedVec::new();
        for p in &additional_payments {
            let farm_token_nonce = self.get_farm_token_nonce_from_attributes(p.token_nonce)?;
            let farm_tokens_payment =
                EsdtTokenPayment::new(farm_token_id.clone(), farm_token_nonce, p.amount.clone());

            farm_tokens.push(farm_tokens_payment);

            self.burn_dual_yield_tokens(p.token_nonce, &p.amount);
        }

        let staking_token_amount = self.get_lp_tokens_value_in_staking_token(&first_payment.amount);
        let staking_farm_address = self.staking_farm_address().get();
        let received_farm_token: EnterFarmResultType<Self::Api> = self
            .staking_farm_proxy_obj(staking_farm_address)
            .stake_farm_through_proxy(farm_tokens, staking_token_amount)
            .execute_on_dest_context();

        let caller = self.blockchain().get_caller();
        self.create_and_send_dual_yield_tokens(
            &caller,
            &received_farm_token.amount,
            received_farm_token.token_nonce,
        );

        Ok(())
    }

    // TODO: Call some method in the pair contract
    fn get_lp_tokens_value_in_staking_token(&self, lp_tokens_amount: &BigUint) -> BigUint {
        lp_tokens_amount.clone()
    }

    // proxies

    #[proxy]
    fn staking_farm_proxy_obj(&self, sc_address: ManagedAddress) -> farm_staking::Proxy<Self::Api>;

    // storage

    #[view(getStakingFarmAddress)]
    #[storage_mapper("stakingFarmAddress")]
    fn staking_farm_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPairAddress)]
    #[storage_mapper("pairAddress")]
    fn pair_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getStakingTokenId)]
    #[storage_mapper("stakingTokenId")]
    fn staking_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLpTokenId)]
    #[storage_mapper("lpTokenId")]
    fn lp_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farmTokenId")]
    fn farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
