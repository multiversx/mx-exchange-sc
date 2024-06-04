use multiversx_sc::proxy_imports::*;

pub struct FeesCollectorProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for FeesCollectorProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = FeesCollectorProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        FeesCollectorProxyMethods { wrapped_tx: tx }
    }
}

pub struct FeesCollectorProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

impl<Env, From, To, Gas> FeesCollectorProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn last_global_update_week(self) -> TxProxyCall<Env, From, To, Gas, usize> {
        self.wrapped_tx
            .raw_call("getLastGlobalUpdateWeek")
            .original_result()
    }

    pub fn total_energy_for_week<Arg0: ProxyArg<usize>>(
        self,
        week: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, BigUint<Env::Api>> {
        self.wrapped_tx
            .raw_call("getTotalEnergyForWeek")
            .argument(&week)
            .original_result()
    }
}
