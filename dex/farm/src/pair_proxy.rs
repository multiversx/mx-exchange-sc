use multiversx_sc::proxy_imports::*;

pub struct PairProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for PairProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = PairProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        PairProxyMethods { wrapped_tx: tx }
    }
}

pub struct PairProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

impl<Env, From, To, Gas> PairProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn remove_liquidity_and_burn_token<Arg0: ProxyArg<TokenIdentifier<Env::Api>>>(
        self,
        token_to_buyback_and_burn: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("removeLiquidityAndBuyBackAndBurnToken")
            .argument(&token_to_buyback_and_burn)
            .original_result()
    }
}
