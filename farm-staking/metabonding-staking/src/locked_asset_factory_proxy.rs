use multiversx_sc::proxy_imports::*;

pub struct LockedAssetFactoryProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for LockedAssetFactoryProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = LockedAssetFactoryProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        LockedAssetFactoryProxyMethods { wrapped_tx: tx }
    }
}

pub struct LockedAssetFactoryProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

#[rustfmt::skip]
impl<Env, From, To, Gas> LockedAssetFactoryProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn merge_tokens(
        self,
    ) -> TxProxyCall<Env, From, To, Gas, EsdtTokenPayment<Env::Api>> {
        self.wrapped_tx
            .raw_call("mergeTokens")
            .original_result()
    }
}
