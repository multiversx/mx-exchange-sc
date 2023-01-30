multiversx_sc::imports!();

use pair::{AddLiquidityResultType, ProxyTrait as _, RemoveLiquidityResultType};

pub struct AddLiquidityResultWrapper<M: ManagedTypeApi> {
    pub lp_tokens_received: EsdtTokenPayment<M>,
    pub first_token_leftover: EsdtTokenPayment<M>,
    pub second_token_leftover: EsdtTokenPayment<M>,
}

pub struct RemoveLiqudityResultWrapper<M: ManagedTypeApi> {
    pub first_token_received: EsdtTokenPayment<M>,
    pub second_token_received: EsdtTokenPayment<M>,
}

#[multiversx_sc::module]
pub trait PairInteractionsModule {
    fn call_add_liquidity(
        &self,
        pair_address: ManagedAddress,
        first_token_id: TokenIdentifier,
        first_token_amount_desired: BigUint,
        first_token_amount_min: BigUint,
        second_token_id: TokenIdentifier,
        second_token_amount_desired: BigUint,
        second_token_amount_min: BigUint,
    ) -> AddLiquidityResultWrapper<Self::Api> {
        let first_payment =
            EsdtTokenPayment::new(first_token_id, 0, first_token_amount_desired.clone());
        let second_payment =
            EsdtTokenPayment::new(second_token_id, 0, second_token_amount_desired.clone());

        let mut all_token_payments = ManagedVec::new();
        all_token_payments.push(first_payment);
        all_token_payments.push(second_payment);

        let raw_result: AddLiquidityResultType<Self::Api> = self
            .pair_contract_proxy(pair_address)
            .add_liquidity(first_token_amount_min, second_token_amount_min)
            .with_multi_token_transfer(all_token_payments)
            .execute_on_dest_context();
        let (lp_tokens_received, first_tokens_used, second_tokens_used) = raw_result.into_tuple();
        let first_token_leftover_amount = &first_token_amount_desired - &first_tokens_used.amount;
        let second_token_leftover_amount =
            &second_token_amount_desired - &second_tokens_used.amount;

        let first_token_leftover = EsdtTokenPayment::new(
            first_tokens_used.token_identifier,
            0,
            first_token_leftover_amount,
        );
        let second_token_leftover = EsdtTokenPayment::new(
            second_tokens_used.token_identifier,
            0,
            second_token_leftover_amount,
        );

        AddLiquidityResultWrapper {
            lp_tokens_received,
            first_token_leftover,
            second_token_leftover,
        }
    }

    fn call_remove_liquidity(
        &self,
        pair_address: ManagedAddress,
        lp_token_id: TokenIdentifier,
        lp_token_amount: BigUint,
        first_token_amount_min: BigUint,
        second_token_amount_min: BigUint,
    ) -> RemoveLiqudityResultWrapper<Self::Api> {
        let raw_result: RemoveLiquidityResultType<Self::Api> = self
            .pair_contract_proxy(pair_address)
            .remove_liquidity(first_token_amount_min, second_token_amount_min)
            .with_esdt_transfer((lp_token_id, 0, lp_token_amount))
            .execute_on_dest_context();
        let (first_token_received, second_token_received) = raw_result.into_tuple();

        RemoveLiqudityResultWrapper {
            first_token_received,
            second_token_received,
        }
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;
}
