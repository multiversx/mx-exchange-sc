elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use token_merge::ValueWeight;

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct StakingFarmTokenAttributes<M: ManagedTypeApi> {
    pub reward_per_share: BigUint<M>,
    pub entering_epoch: u64,
    pub last_claim_block: u64,
    pub initial_farming_amount: BigUint<M>,
    pub compounded_reward: BigUint<M>,
    pub current_farm_amount: BigUint<M>,
}

#[derive(ManagedVecItem, Clone)]
pub struct StakingFarmToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: StakingFarmTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait FarmTokenMergeModule:
    token_send::TokenSendModule
    + farm_token::FarmTokenModule
    + config::ConfigModule
    + token_merge::TokenMergeModule
{
    #[payable("*")]
    #[endpoint(mergeFarmTokens)]
    fn merge_farm_tokens(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<ManagedBuffer>,
    ) -> SCResult<EsdtTokenPayment<Self::Api>> {
        let caller = self.blockchain().get_caller();
        let payments = self.get_all_payments_managed_vec();

        let attrs = self.get_merged_farm_token_attributes(&payments, None, None)?;
        let farm_token_id = self.farm_token_id().get();
        self.burn_farm_tokens_from_payments(&payments);

        let new_nonce = self.mint_farm_tokens(&farm_token_id, &attrs.current_farm_amount, &attrs);
        let new_amount = attrs.current_farm_amount;

        self.transfer_execute_custom(
            &caller,
            &farm_token_id,
            new_nonce,
            &new_amount,
            &opt_accept_funds_func,
        )?;

        Ok(self.create_payment(&farm_token_id, new_nonce, &new_amount))
    }

    fn get_merged_farm_token_attributes(
        &self,
        payments: &ManagedVec<EsdtTokenPayment<Self::Api>>,
        replic: Option<StakingFarmToken<Self::Api>>,
        opt_custom_attributes_for_payments: Option<
            &ManagedVec<StakingFarmTokenAttributes<Self::Api>>,
        >,
    ) -> SCResult<StakingFarmTokenAttributes<Self::Api>> {
        require!(
            !payments.is_empty() || replic.is_some(),
            "No tokens to merge"
        );

        let mut tokens = ManagedVec::new();
        let farm_token_id = self.farm_token_id().get();
        let empty_vec = ManagedVec::new();
        let custom_attributes = opt_custom_attributes_for_payments.unwrap_or(&empty_vec);

        for (i, payment) in payments.iter().enumerate() {
            require!(payment.amount != 0u64, "zero entry amount");
            require!(
                payment.token_identifier == farm_token_id,
                "Not a farm token"
            );

            let attributes = match custom_attributes.get(i) {
                Some(attr) => attr,
                None => self.get_attributes(&payment.token_identifier, payment.token_nonce)?,
            };
            tokens.push(StakingFarmToken {
                token_amount: self.create_payment(
                    &payment.token_identifier,
                    payment.token_nonce,
                    &payment.amount,
                ),
                attributes,
            });
        }

        if let Some(r) = replic {
            tokens.push(r);
        }

        if tokens.len() == 1 {
            if let Some(t) = tokens.get(0) {
                return Ok(t.attributes);
            }
        }

        let current_epoch = self.blockchain().get_block_epoch();
        let current_block = self.blockchain().get_block_nonce();
        let aggregated_attributes = StakingFarmTokenAttributes {
            reward_per_share: self.aggregated_reward_per_share(&tokens),
            entering_epoch: current_epoch,
            last_claim_block: current_block,
            initial_farming_amount: self.aggregated_initial_farming_amount(&tokens)?,
            compounded_reward: self.aggregated_compounded_reward(&tokens),
            current_farm_amount: self.aggregated_current_farm_amount(&tokens),
        };

        Ok(aggregated_attributes)
    }

    fn aggregated_reward_per_share(
        &self,
        tokens: &ManagedVec<StakingFarmToken<Self::Api>>,
    ) -> BigUint {
        let mut dataset = ManagedVec::new();
        tokens.iter().for_each(|x| {
            dataset.push(ValueWeight {
                value: x.attributes.reward_per_share.clone(),
                weight: x.token_amount.amount.clone(),
            })
        });
        self.weighted_average_ceil(dataset)
    }

    fn aggregated_initial_farming_amount(
        &self,
        tokens: &ManagedVec<StakingFarmToken<Self::Api>>,
    ) -> SCResult<BigUint> {
        let mut sum = BigUint::zero();
        for x in tokens.iter() {
            sum += &self.rule_of_three_non_zero_result(
                &x.token_amount.amount,
                &x.attributes.current_farm_amount,
                &x.attributes.initial_farming_amount,
            );
        }
        Ok(sum)
    }

    fn aggregated_compounded_reward(
        &self,
        tokens: &ManagedVec<StakingFarmToken<Self::Api>>,
    ) -> BigUint {
        let mut sum = BigUint::zero();
        tokens.iter().for_each(|x| {
            sum += &self.rule_of_three(
                &x.token_amount.amount,
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
            .for_each(|x| aggregated_amount += &x.token_amount.amount);
        aggregated_amount
    }

    fn get_attributes<T: TopDecode>(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<T> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        token_info.decode_attributes().into()
    }
}
