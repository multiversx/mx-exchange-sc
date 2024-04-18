#![allow(clippy::type_complexity)]

use multiversx_sc::proxy_imports::*;

pub struct LpProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for LpProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = LpProxyProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        LpProxyProxyMethods { wrapped_tx: tx }
    }
}

pub struct LpProxyProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

#[rustfmt::skip]
impl<Env, From, To, Gas> LpProxyProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn add_liquidity<
        Arg0: CodecInto<BigUint<Env::Api>>,
        Arg1: CodecInto<BigUint<Env::Api>>,
    >(
        self,
        first_token_amount_min: Arg0,
        second_token_amount_min: Arg1,
    ) -> TxProxyCall<Env, From, To, Gas, MultiValue3<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>> {
        self.wrapped_tx
            .raw_call("addLiquidity")
            .argument(&first_token_amount_min)
            .argument(&second_token_amount_min)
            .original_result()
    }

    pub fn remove_liquidity<
        Arg0: CodecInto<BigUint<Env::Api>>,
        Arg1: CodecInto<BigUint<Env::Api>>,
    >(
        self,
        first_token_amount_min: Arg0,
        second_token_amount_min: Arg1,
    ) -> TxProxyCall<Env, From, To, Gas, MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>> {
        self.wrapped_tx
            .raw_call("removeLiquidity")
            .argument(&first_token_amount_min)
            .argument(&second_token_amount_min)
            .original_result()
    }
}
