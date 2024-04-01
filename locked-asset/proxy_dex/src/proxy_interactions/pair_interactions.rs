multiversx_sc::imports!();

use pair::pair_actions::{
    add_liq::ProxyTrait as _,
    common_result_types::{AddLiquidityResultType, RemoveLiquidityResultType},
    initial_liq::ProxyTrait as _,
    remove_liq::ProxyTrait as _,
};

pub struct AddInitialLiqArgs<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub first_token_amount_desired: BigUint<M>,
    pub second_token_id: TokenIdentifier<M>,
    pub second_token_amount_desired: BigUint<M>,
}

pub struct AddLiqArgs<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub first_token_id: TokenIdentifier<M>,
    pub first_token_amount_desired: BigUint<M>,
    pub first_token_amount_min: BigUint<M>,
    pub second_token_id: TokenIdentifier<M>,
    pub second_token_amount_desired: BigUint<M>,
    pub second_token_amount_min: BigUint<M>,
}

pub struct RemoveLiqArgs<M: ManagedTypeApi> {
    pub pair_address: ManagedAddress<M>,
    pub lp_token_id: TokenIdentifier<M>,
    pub lp_token_amount: BigUint<M>,
    pub first_token_amount_min: BigUint<M>,
    pub second_token_amount_min: BigUint<M>,
}

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
    fn call_add_initial_liq(
        &self,
        args: AddInitialLiqArgs<Self::Api>,
    ) -> AddLiquidityResultWrapper<Self::Api> {
        let first_payment = EsdtTokenPayment::new(
            args.first_token_id,
            0,
            args.first_token_amount_desired.clone(),
        );
        let second_payment = EsdtTokenPayment::new(
            args.second_token_id,
            0,
            args.second_token_amount_desired.clone(),
        );

        let mut all_token_payments = ManagedVec::new();
        all_token_payments.push(first_payment);
        all_token_payments.push(second_payment);

        let raw_result: AddLiquidityResultType<Self::Api> = self
            .pair_contract_proxy(args.pair_address)
            .add_initial_liquidity()
            .with_multi_token_transfer(all_token_payments)
            .execute_on_dest_context();
        let (lp_tokens_received, first_tokens_used, second_tokens_used) = raw_result.into_tuple();
        let first_token_leftover_amount =
            &args.first_token_amount_desired - &first_tokens_used.amount;
        let second_token_leftover_amount =
            &args.second_token_amount_desired - &second_tokens_used.amount;

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

    fn call_add_liquidity(
        &self,
        args: AddLiqArgs<Self::Api>,
    ) -> AddLiquidityResultWrapper<Self::Api> {
        let first_payment = EsdtTokenPayment::new(
            args.first_token_id,
            0,
            args.first_token_amount_desired.clone(),
        );
        let second_payment = EsdtTokenPayment::new(
            args.second_token_id,
            0,
            args.second_token_amount_desired.clone(),
        );

        let mut all_token_payments = ManagedVec::new();
        all_token_payments.push(first_payment);
        all_token_payments.push(second_payment);

        let raw_result: AddLiquidityResultType<Self::Api> = self
            .pair_contract_proxy(args.pair_address)
            .add_liquidity(args.first_token_amount_min, args.second_token_amount_min)
            .with_multi_token_transfer(all_token_payments)
            .execute_on_dest_context();
        let (lp_tokens_received, first_tokens_used, second_tokens_used) = raw_result.into_tuple();
        let first_token_leftover_amount =
            &args.first_token_amount_desired - &first_tokens_used.amount;
        let second_token_leftover_amount =
            &args.second_token_amount_desired - &second_tokens_used.amount;

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
        args: RemoveLiqArgs<Self::Api>,
    ) -> RemoveLiqudityResultWrapper<Self::Api> {
        let raw_result: RemoveLiquidityResultType<Self::Api> = self
            .pair_contract_proxy(args.pair_address)
            .remove_liquidity(args.first_token_amount_min, args.second_token_amount_min)
            .with_esdt_transfer((args.lp_token_id, 0, args.lp_token_amount))
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
