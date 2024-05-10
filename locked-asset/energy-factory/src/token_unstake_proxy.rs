use multiversx_sc::proxy_imports::*;

pub struct TokenUnstakeProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for TokenUnstakeProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = TokenUnstakeProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        TokenUnstakeProxyMethods { wrapped_tx: tx }
    }
}

pub struct TokenUnstakeProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

#[rustfmt::skip]
impl<Env, From, To, Gas> TokenUnstakeProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn deposit_user_tokens<
        Arg0: ProxyArg<ManagedAddress<Env::Api>>,
    >(
        self,
        user: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("depositUserTokens")
            .argument(&user)
            .original_result()
    }

    pub fn deposit_fees(
        self,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("depositFees")
            .original_result()
    }
}
