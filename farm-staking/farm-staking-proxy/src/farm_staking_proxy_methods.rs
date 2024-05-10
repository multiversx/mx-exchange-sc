#![allow(clippy::type_complexity)]

use multiversx_sc::proxy_imports::*;

pub struct FarmStakingProxy;

impl<Env, From, To, Gas> TxProxyTrait<Env, From, To, Gas> for FarmStakingProxy
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    type TxProxyMethods = FarmStakingProxyMethods<Env, From, To, Gas>;

    fn proxy_methods(self, tx: Tx<Env, From, To, (), Gas, (), ()>) -> Self::TxProxyMethods {
        FarmStakingProxyMethods { wrapped_tx: tx }
    }
}

pub struct FarmStakingProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    wrapped_tx: Tx<Env, From, To, (), Gas, (), ()>,
}

impl<Env, From, To, Gas> FarmStakingProxyMethods<Env, From, To, Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    To: TxTo<Env>,
    Gas: TxGas<Env>,
{
    pub fn stake_farm_through_proxy<
        Arg0: CodecInto<BigUint<Env::Api>>,
        Arg1: CodecInto<ManagedAddress<Env::Api>>,
    >(
        self,
        staked_token_amount: Arg0,
        original_caller: Arg1,
    ) -> TxProxyCall<
        Env,
        From,
        To,
        Gas,
        MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>,
    > {
        self.wrapped_tx
            .raw_call("stakeFarmThroughProxy")
            .argument(&staked_token_amount)
            .argument(&original_caller)
            .original_result()
    }

    pub fn claim_rewards_with_new_value<
        Arg0: CodecInto<BigUint<Env::Api>>,
        Arg1: CodecInto<ManagedAddress<Env::Api>>,
    >(
        self,
        new_farming_amount: Arg0,
        original_caller: Arg1,
    ) -> TxProxyCall<
        Env,
        From,
        To,
        Gas,
        MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>,
    > {
        self.wrapped_tx
            .raw_call("claimRewardsWithNewValue")
            .argument(&new_farming_amount)
            .argument(&original_caller)
            .original_result()
    }

    pub fn unstake_farm_through_proxy<Arg0: CodecInto<ManagedAddress<Env::Api>>>(
        self,
        original_caller: Arg0,
    ) -> TxProxyCall<
        Env,
        From,
        To,
        Gas,
        MultiValue2<EsdtTokenPayment<Env::Api>, EsdtTokenPayment<Env::Api>>,
    > {
        self.wrapped_tx
            .raw_call("unstakeFarmThroughProxy")
            .argument(&original_caller)
            .original_result()
    }
}
