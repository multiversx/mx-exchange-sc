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

#[rustfmt::skip]
impl<Env, From, Gas> FarmProxyMethods<Env, From, (), Gas>
where
    Env: TxEnv,
    Env::Api: VMApi,
    From: TxFrom<Env>,
    Gas: TxGas<Env>,
{
    pub fn init<
        Arg0: CodecInto<TokenIdentifier<Env::Api>>,
        Arg1: CodecInto<TokenIdentifier<Env::Api>>,
        Arg2: CodecInto<BigUint<Env::Api>>,
        Arg3: CodecInto<ManagedAddress<Env::Api>>,
        Arg4: CodecInto<ManagedAddress<Env::Api>>,
        Arg5: CodecInto<MultiValueEncoded<Env::Api, ManagedAddress<Env::Api>>>,
    >(
        self,
        reward_token_id: Arg0,
        farming_token_id: Arg1,
        division_safety_constant: Arg2,
        pair_contract_address: Arg3,
        owner: Arg4,
        admins: Arg5,
    ) -> TxProxyDeploy<Env, From, Gas, ()> {
        self.wrapped_tx
            .raw_deploy()
            .argument(&reward_token_id)
            .argument(&farming_token_id)
            .argument(&division_safety_constant)
            .argument(&pair_contract_address)
            .argument(&owner)
            .argument(&admins)
            .original_result()
    }
}
