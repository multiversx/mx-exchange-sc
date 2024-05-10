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
    pub fn update_energy_after_old_token_unlock<
        Arg0: CodecInto<ManagedAddress<Env::Api>>,
        Arg1: CodecInto<common_structs::locked_token_types::UnlockEpochAmountPairs<Env::Api>>,
        Arg2: CodecInto<common_structs::locked_token_types::UnlockEpochAmountPairs<Env::Api>>,
    >(
        self,
        original_caller: Arg0,
        initial_epoch_amount_pairs: Arg1,
        final_epoch_amount_pairs: Arg2,
    ) -> TxProxyCall<Env, From, To, Gas, ()> {
        self.wrapped_tx
            .raw_call("updateEnergyAfterOldTokenUnlock")
            .argument(&original_caller)
            .argument(&initial_epoch_amount_pairs)
            .argument(&final_epoch_amount_pairs)
            .original_result()
    }
}
