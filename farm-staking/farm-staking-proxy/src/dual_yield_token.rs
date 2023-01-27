use elrond_wasm::elrond_codec::NestedDecodeInput;
use fixed_supply_token::FixedSupplyToken;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[derive(TypeAbi, TopEncode, PartialEq, Debug, Clone)]
pub struct DualYieldTokenAttributes<M: ManagedTypeApi> {
    pub lp_farm_token_nonce: u64,
    pub lp_farm_token_amount: BigUint<M>,
    pub staking_farm_token_nonce: u64,
    pub staking_farm_token_amount: BigUint<M>,
    pub user_staking_farm_token_amount: BigUint<M>,
}

impl<M: ManagedTypeApi> DualYieldTokenAttributes<M> {
    pub fn new(
        lp_farm_token_nonce: u64,
        lp_farm_token_amount: BigUint<M>,
        staking_farm_token_nonce: u64,
        staking_farm_token_amount: BigUint<M>,
    ) -> Self {
        DualYieldTokenAttributes {
            lp_farm_token_nonce,
            lp_farm_token_amount,
            staking_farm_token_nonce,
            staking_farm_token_amount,
            user_staking_farm_token_amount: BigUint::zero(),
        }
    }

    pub fn get_total_staking_token_amount(&self) -> BigUint<M> {
        &self.staking_farm_token_amount + &self.user_staking_farm_token_amount
    }
}

impl<M: ManagedTypeApi> TopDecode for DualYieldTokenAttributes<M> {
    fn top_decode<I>(input: I) -> Result<Self, DecodeError>
    where
        I: elrond_codec::TopDecodeInput,
    {
        let mut buffer = input.into_nested_buffer();
        Self::dep_decode(&mut buffer)
    }
}

impl<M: ManagedTypeApi> NestedDecode for DualYieldTokenAttributes<M> {
    fn dep_decode<I: NestedDecodeInput>(input: &mut I) -> Result<Self, DecodeError> {
        let lp_farm_token_nonce = u64::dep_decode(input)?;
        let lp_farm_token_amount = BigUint::dep_decode(input)?;
        let staking_farm_token_nonce = u64::dep_decode(input)?;
        let staking_farm_token_amount = BigUint::dep_decode(input)?;

        if input.is_depleted() {
            return Result::Ok(DualYieldTokenAttributes::new(
                lp_farm_token_nonce,
                lp_farm_token_amount,
                staking_farm_token_nonce,
                staking_farm_token_amount,
            ));
        }

        let user_staking_farm_token_amount = BigUint::dep_decode(input)?;

        if !input.is_depleted() {
            return Result::Err(DecodeError::INPUT_TOO_LONG);
        }

        Result::Ok(DualYieldTokenAttributes {
            lp_farm_token_nonce,
            lp_farm_token_amount,
            staking_farm_token_nonce,
            staking_farm_token_amount,
            user_staking_farm_token_amount,
        })
    }
}

impl<M: ManagedTypeApi> FixedSupplyToken<M> for DualYieldTokenAttributes<M> {
    fn get_total_supply(&self) -> BigUint<M> {
        self.staking_farm_token_amount.clone()
    }

    fn into_part(self, payment_amount: &BigUint<M>) -> Self {
        if payment_amount == &self.get_total_supply() {
            return self;
        }

        let new_lp_farm_token_amount =
            self.rule_of_three_non_zero_result(payment_amount, &self.lp_farm_token_amount);
        let new_staking_farm_token_amount = payment_amount.clone();
        let new_additional_token_amount =
            self.rule_of_three(payment_amount, &self.user_staking_farm_token_amount);

        DualYieldTokenAttributes {
            lp_farm_token_nonce: self.lp_farm_token_nonce,
            lp_farm_token_amount: new_lp_farm_token_amount,
            staking_farm_token_nonce: self.staking_farm_token_nonce,
            staking_farm_token_amount: new_staking_farm_token_amount,
            user_staking_farm_token_amount: new_additional_token_amount,
        }
    }
}

#[elrond_wasm::module]
pub trait DualYieldTokenModule:
    elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerDualYieldToken)]
    fn register_dual_yield_token(
        &self,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        let register_cost = self.call_value().egld_value();
        self.dual_yield_token().issue_and_set_all_roles(
            EsdtTokenType::Meta,
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
            None,
        );
    }

    fn create_dual_yield_tokens(
        &self,
        mapper: &NonFungibleTokenMapper,
        attributes: &DualYieldTokenAttributes<Self::Api>,
    ) -> EsdtTokenPayment {
        let new_dual_yield_amount = attributes.get_total_supply();
        mapper.nft_create(new_dual_yield_amount, attributes)
    }

    #[view(getDualYieldTokenId)]
    #[storage_mapper("dualYieldTokenId")]
    fn dual_yield_token(&self) -> NonFungibleTokenMapper;
}
