use energy_query::Energy;
use multiversx_sc::proxy_imports::*;

pub struct SimpleLockEnergyProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for SimpleLockEnergyProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = SimpleLockEnergyProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        SimpleLockEnergyProxyMethods { wrapped_tx: tx }
    }
}

pub struct SimpleLockEnergyProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

#[rustfmt::skip]
impl<Env, From, To, Gas> SimpleLockEnergyProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn revert_unstake<
        Arg0: CodecInto<ManagedAddress<Env::Api>>,
        Arg1: CodecInto<Energy<Env::Api>>,
    >(
        self,
        user: Arg0,
        new_energy: Arg1,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("revertUnstake")
            .argument(&user)
            .argument(&new_energy)
            .original_result()
    }
}
