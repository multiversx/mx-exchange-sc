use multiversx_sc::proxy_imports::*;

pub struct PriceProviderProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for PriceProviderProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = PriceProviderProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        PriceProviderProxyMethods { wrapped_tx: tx }
    }
}

pub struct PriceProviderProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

impl<Env, From, To, Gas> PriceProviderProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn get_tokens_for_given_position_with_safe_price<Arg0: ProxyArg<BigUint<Env::Api>>>(
        self,
        liquidity: Arg0,
    ) -> TxProxyCall<
        Env,
        From,
        To,
        Gas,
        MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>,
    > {
        self.wrapped_tx
            .raw_call("getTokensForGivenPositionWithSafePrice")
            .argument(&liquidity)
            .original_result()
    }
}
