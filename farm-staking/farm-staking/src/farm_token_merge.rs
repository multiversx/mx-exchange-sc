elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use token_merge::ValueWeight;

#[derive(
    ManagedVecItem,
    TopEncode,
    TopDecode,
    NestedEncode,
    NestedDecode,
    TypeAbi,
    Clone,
    PartialEq,
    Debug,
)]
pub struct StakingFarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[derive(ManagedVecItem, Clone)]
pub struct StakingFarmToken<M: ManagedTypeApi> {
    pub payment: EsdtTokenPayment<M>,
    pub attributes: StakingFarmTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait FarmTokenMergeModule:
    token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + config::ConfigModule
    + token_merge::TokenMergeModule
    + pausable::PausableModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(&self) -> EsdtTokenPayment<Self::Api> {
        let caller = self.blockchain().get_caller();
        let payments = self.call_value().all_esdt_transfers();

        let attrs = self.get_merged_farm_token_attributes(&payments, None);

        self.burn_farm_tokens_from_payments(&payments);

        self.farm_token()
            .nft_create_and_send(&caller, attrs.current_farm_amount.clone(), &attrs)
    }

    fn get_merged_farm_token_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        replic: Option<StakingFarmToken<Self::Api>>,
    ) -> StakingFarmTokenAttributes<Self::Api> {
        require!(
            !payments.is_empty() || replic.is_some(),
            "No tokens to merge"
        );

        let farm_token_id = match &replic {
            Some(r) => r.payment.token_identifier.clone(),
            None => self.farm_token().get_token_id(),
        };

        let mut tokens = ManagedVec::new();
        for payment in payments {
            require!(payment.amount != 0u64, "zero entry amount");
            require!(
                payment.token_identifier == farm_token_id,
                "Not a farm token"
            );

            tokens.push(StakingFarmToken {
                attributes: self.get_attributes(&payment.token_identifier, payment.token_nonce),
                payment,
            });
        }

        if let Some(r) = replic {
            tokens.push(r);
        }

        if tokens.len() == 1 {
            if let Some(t) = tokens.try_get(0) {
                return t.attributes;
            }
        }

        StakingFarmTokenAttributes {
            reward_per_share: self.aggregated_reward_per_share(&tokens),
            compounded_reward: self.aggregated_compounded_reward(&tokens),
            current_farm_amount: self.aggregated_current_farm_amount(&tokens),
        }
    }

    fn aggregated_reward_per_share(
        &self,
        tokens: &ManagedVec<StakingFarmToken<Self::Api>>,
    ) -> BigUint {
        let mut dataset = ManagedVec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: x.attributes.reward_per_share.clone(),
                weight: x.payment.amount,
            })
        });
        self.weighted_average_ceil(dataset)
    }

    fn aggregated_compounded_reward(
        &self,
        tokens: &ManagedVec<StakingFarmToken<Self::Api>>,
    ) -> BigUint {
        let mut sum = BigUint::zero();
        tokens.iter().for_each(|x| {
            sum += &self.rule_of_three(
                &x.payment.amount,
                &x.attributes.current_farm_amount,
                &x.attributes.compounded_reward,
            )
        });
        sum
    }

    fn aggregated_current_farm_amount(
        &self,
        tokens: &ManagedVec<StakingFarmToken<Self::Api>>,
    ) -> BigUint {
        let mut aggregated_amount = BigUint::zero();
        tokens
            .iter()
            .for_each(|x| aggregated_amount += &x.payment.amount);
        aggregated_amount
    }

    fn get_attributes<T: TopDecode>(&self, token_id: &TokenIdentifier, token_nonce: u64) -> T {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        token_info.decode_attributes()
    }
}
