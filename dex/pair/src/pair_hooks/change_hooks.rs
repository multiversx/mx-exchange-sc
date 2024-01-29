use super::hook_type::{Hook, HookType};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait ChangeHooksModule:
    super::call_hook::CallHookModule
    + super::banned_address::BannedAddressModule
    + permissions_module::PermissionsModule
    + utils::UtilsModule
{
    #[endpoint(addHook)]
    fn add_hook(&self, hook_type: HookType, to: ManagedAddress, endpoint_name: ManagedBuffer) {
        self.require_caller_has_owner_or_admin_permissions();
        self.require_not_banned_address(&to);
        self.require_sc_address(&to);
        self.require_not_empty_buffer(&endpoint_name);

        self.hooks(hook_type).update(|hooks| {
            hooks.push(Hook {
                dest_address: to,
                endpoint_name,
            })
        });
    }

    #[endpoint(removeHook)]
    fn remove_hook(&self, hook_type: HookType, to: ManagedAddress, endpoint_name: ManagedBuffer) {
        self.require_caller_has_owner_or_admin_permissions();

        self.hooks(hook_type).update(|hooks| {
            let opt_index = hooks.find(&Hook {
                dest_address: to,
                endpoint_name,
            });

            require!(opt_index.is_some(), "Item not found");

            let index = unsafe { opt_index.unwrap_unchecked() };
            hooks.remove(index);
        })
    }
}
