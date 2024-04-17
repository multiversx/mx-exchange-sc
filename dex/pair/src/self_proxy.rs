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
    pub fn swap_no_fee<
        Arg0: CodecInto<TokenIdentifier<Env::Api>>,
        Arg1: CodecInto<ManagedAddress<Env::Api>>,
    >(
        self,
        token_out: Arg0,
        destination_address: Arg1,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("swapNoFeeAndForward")
            .argument(&token_out)
            .argument(&destination_address)
            .original_result()
    }
}
