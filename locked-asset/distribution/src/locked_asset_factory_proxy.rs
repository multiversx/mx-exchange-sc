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
    pub fn create_and_forward_custom_period<
        Arg0: CodecInto<BigUint<Env::Api>>,
        Arg1: CodecInto<ManagedAddress<Env::Api>>,
        Arg2: CodecInto<u64>,
        Arg3: CodecInto<common_structs::locked_token_types::UnlockSchedule<Env::Api>>,
    >(
        self,
        amount: Arg0,
        address: Arg1,
        start_epoch: Arg2,
        unlock_period: Arg3,
    ) -> TxProxyCall<Env, From, To, Gas, EsdtTokenPayment<Env::Api>> {
        self.wrapped_tx
            .raw_call("createAndForwardCustomPeriod")
            .argument(&amount)
            .argument(&address)
            .argument(&start_epoch)
            .argument(&unlock_period)
            .original_result()
    }
}
