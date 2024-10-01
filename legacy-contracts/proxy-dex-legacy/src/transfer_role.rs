multiversx_sc::imports!();

use super::proxy_common;

#[multiversx_sc::module]
pub trait TransferRoleModule: proxy_common::ProxyCommonModule {
    /// Sets the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedLpToken)]
    fn set_transfer_role_locked_lp_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_lp_token_id = self.wrapped_lp_token_id().get();
        let role_dest_address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };
        let roles = [EsdtLocalRole::Transfer];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &role_dest_address,
                &locked_lp_token_id,
                roles.iter().cloned(),
            )
            .async_call_and_exit()
    }

    /// Removes the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(unsetTransferRoleLockedLpToken)]
    fn unset_transfer_role_locked_lp_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_lp_token_id = self.wrapped_lp_token_id().get();
        let role_dest_address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };
        let roles = [EsdtLocalRole::Transfer];

        self.send()
            .esdt_system_sc_proxy()
            .unset_special_roles(
                &role_dest_address,
                &locked_lp_token_id,
                roles.iter().cloned(),
            )
            .async_call_and_exit()
    }

    /// Sets the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(setTransferRoleLockedFarmToken)]
    fn set_transfer_role_locked_farm_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_farm_token_id = self.wrapped_farm_token_id().get();
        let role_dest_address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };
        let roles = [EsdtLocalRole::Transfer];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &role_dest_address,
                &locked_farm_token_id,
                roles.iter().cloned(),
            )
            .async_call_and_exit()
    }

    /// Removes the transfer role for the given address. Defaults to own SC address.
    #[only_owner]
    #[endpoint(unsetTransferRoleLockedFarmToken)]
    fn unset_transfer_role_locked_farm_token(&self, opt_address: OptionalValue<ManagedAddress>) {
        let locked_farm_token_id = self.wrapped_farm_token_id().get();
        let role_dest_address = match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        };
        let roles = [EsdtLocalRole::Transfer];

        self.send()
            .esdt_system_sc_proxy()
            .unset_special_roles(
                &role_dest_address,
                &locked_farm_token_id,
                roles.iter().cloned(),
            )
            .async_call_and_exit()
    }
}
