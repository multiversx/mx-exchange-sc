elrond_wasm::imports!();

use elrond_wasm::{
    api::{CallTypeApi, StorageMapperApi},
    storage::StorageKey,
};

pub struct FungibleToken<SA>
where
    SA: StorageMapperApi + CallTypeApi,
{
    base_key: StorageKey<SA>,
}

impl<SA> StorageMapper<SA> for FungibleToken<SA>
where
    SA: StorageMapperApi + CallTypeApi,
{
    fn new(base_key: StorageKey<SA>) -> Self {
        Self { base_key }
    }
}

impl<SA> FungibleToken<SA>
where
    SA: StorageMapperApi + CallTypeApi,
{
    pub fn issue(
        issue_cost: BigUint<SA>,
        token_display_name: ManagedBuffer<SA>,
        token_ticker: ManagedBuffer<SA>,
        num_decimals: usize,
        opt_callback: Option<CallbackClosure<SA>>
    ) -> AsyncCall<SA> {
        let system_sc_proxy = ESDTSystemSmartContractProxy::<SA>::new_proxy_obj();
        let mut async_call = system_sc_proxy.issue_fungible(
            issue_cost,
            &token_display_name,
            &token_ticker,
            &BigUint::zero(),
            FungibleTokenProperties {
                num_decimals,
                can_freeze: true,
                can_wipe: true,
                can_pause: true,
                can_mint: false,
                can_burn: false,
                can_change_owner: true,
                can_upgrade: true,
                can_add_special_roles: true,
            },
        ).async_call();

        if let Some(callback) = opt_callback {
            async_call = async_call.with_callback(callback);
        }

        async_call
    }
}
