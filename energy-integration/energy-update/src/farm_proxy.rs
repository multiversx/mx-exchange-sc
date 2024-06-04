use multiversx_sc::proxy_imports::*;

pub struct FarmProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for FarmProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = FarmProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        FarmProxyMethods { wrapped_tx: tx }
    }
}

pub struct FarmProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

impl<Env, From, To, Gas> FarmProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn update_energy_for_user<Arg0: ProxyArg<ManagedAddress<Env::Api>>>(
        self,
        user: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("updateEnergyForUser")
            .argument(&user)
            .original_result()
    }
}
