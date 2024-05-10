#![allow(clippy::type_complexity)]

use multiversx_sc::proxy_imports::*;

pub struct FarmProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for FarmProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = FarmProxyProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        FarmProxyProxyMethods { wrapped_tx: tx }
    }
}

pub struct FarmProxyProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

#[rustfmt::skip]
impl<Env, From, To, Gas> FarmProxyProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn enter_farm<
        Arg0: ProxyArg<OptionalValue<ManagedAddress<Env::Api>>>,
    >(
        self,
        opt_orig_caller: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>> {
        self.wrapped_tx
            .raw_call("enterFarm")
            .argument(&opt_orig_caller)
            .original_result()
    }

    pub fn exit_farm<
        Arg0: ProxyArg<OptionalValue<ManagedAddress<Env::Api>>>,
    >(
        self,
        opt_orig_caller: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>> {
        self.wrapped_tx
            .raw_call("exitFarm")
            .argument(&opt_orig_caller)
            .original_result()
    }

    pub fn claim_rewards<
        Arg0: ProxyArg<OptionalValue<ManagedAddress<Env::Api>>>,
    >(
        self,
        opt_orig_caller: Arg0,
    ) -> TxProxyCall<Env, From, To, Gas, MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>> {
        self.wrapped_tx
            .raw_call("claimRewards")
            .argument(&opt_orig_caller)
            .original_result()
    }
}
