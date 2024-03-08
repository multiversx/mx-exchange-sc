multiversx_sc::imports!();

use energy_factory::{
    lock_options::{AllLockOptions, MAX_PENALTY_PERCENTAGE},
    virtual_lock::ProxyTrait as _,
};

#[multiversx_sc::module]
pub trait LockWithEnergyModule {
    #[only_owner]
    #[endpoint(setLockingScAddress)]
    fn set_locking_sc_address(&self, new_address: ManagedAddress) {
        require!(
            self.blockchain().is_smart_contract(&new_address),
            "Invalid SC Address"
        );

        self.locking_sc_address().set(&new_address);
    }

    #[only_owner]
    #[endpoint(setLockEpochs)]
    fn set_lock_epochs(&self, lock_epochs: u64) {
        self.lock_epochs().set(lock_epochs);
    }

    fn lock_virtual(
        &self,
        token_id: TokenIdentifier,
        amount: BigUint,
        dest_address: ManagedAddress,
        energy_address: ManagedAddress,
    ) -> EsdtTokenPayment {
        let lock_epochs = self.lock_epochs().get();
        let mut proxy_instance = self.get_locking_sc_proxy_instance();

        proxy_instance
            .lock_virtual(token_id, amount, lock_epochs, dest_address, energy_address)
            .execute_on_dest_context()
    }

    fn get_locking_sc_proxy_instance(&self) -> energy_factory::Proxy<Self::Api> {
        let locking_sc_address = self.locking_sc_address().get();
        self.locking_sc_proxy_obj(locking_sc_address)
    }

    fn apply_unlock_early_penalty(&self, amount: BigUint) -> BigUint {
        if amount == 0 {
            return amount;
        }

        let energy_factory_address = self.locking_sc_address().get();
        let lock_options = self
            .lock_options()
            .get_from_address(&energy_factory_address);

        let lock_epochs = self.lock_epochs().get();
        let mut opt_lock_option = None;
        for lock_option in lock_options {
            if lock_option.lock_epochs == lock_epochs {
                opt_lock_option = Some(lock_option);

                break;
            }
        }

        require!(opt_lock_option.is_some(), "Lock option not found");

        let lock_option = unsafe { opt_lock_option.unwrap_unchecked() };
        let penalty_amount =
            &amount * lock_option.penalty_start_percentage / MAX_PENALTY_PERCENTAGE;

        amount - penalty_amount
    }

    #[proxy]
    fn locking_sc_proxy_obj(&self, sc_address: ManagedAddress) -> energy_factory::Proxy<Self::Api>;

    #[view(getLockingScAddress)]
    #[storage_mapper("lockingScAddress")]
    fn locking_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getLockEpochs)]
    #[storage_mapper("lockEpochs")]
    fn lock_epochs(&self) -> SingleValueMapper<u64>;

    // energy factory storage

    #[storage_mapper("lockOptions")]
    fn lock_options(&self) -> SingleValueMapper<AllLockOptions>;
}
