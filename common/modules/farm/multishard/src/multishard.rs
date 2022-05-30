#![no_std]

use common_errors::ERROR_SIBLING_PERMISSIONS;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait MultishardModule: config::ConfigModule + token_send::TokenSendModule {
    fn synchronize_impl<F: FnOnce()>(&self, generate_rewards: F) {
        let local_supply = self.farm_token_supply().get();
        self.sibling_supplies()
            .insert(self.blockchain().get_sc_address(), local_supply);

        for sibling in self.sibling_whitelist().iter() {
            self.proxy_call_accept_synchronization(sibling);
        }

        self.try_create_checkpoint(generate_rewards);
    }

    fn proxy_call_accept_synchronization(&self, sibling: ManagedAddress) {
        let endpoint_name = ManagedBuffer::new_from_bytes(b"acceptSynchronization");
        let mut accept_sync: ContractCall<<Self as ContractBase>::Api, ()> =
            self.send().contract_call(sibling, endpoint_name);
        let local_supply = self.farm_token_supply().get();
        accept_sync.push_endpoint_arg(&local_supply);
        accept_sync.transfer_execute();
    }

    fn accept_synchronization_impl<F: FnOnce()>(
        &self,
        sibling_supply: BigUint,
        generate_rewards: F,
    ) {
        let sibling = self.blockchain().get_caller();
        self.require_sibling_whitelisted(sibling.clone());
        self.sibling_supplies().insert(sibling, sibling_supply);
        self.try_create_checkpoint(generate_rewards);
    }

    fn try_create_checkpoint<F: FnOnce()>(&self, generate_rewards: F) {
        if self.sibling_supplies().len() != self.sibling_whitelist().len() + 1 {
            return;
        }

        let mut global_liquidity = BigUint::zero();
        for local_liquidity in self.sibling_supplies().values() {
            global_liquidity += local_liquidity;
        }

        self.sibling_supplies().clear();

        let current_nonce = self.blockchain().get_block_nonce();
        self.current_checkpoint_block_nonce().set(current_nonce);
        self.global_farm_token_supply().set(global_liquidity);

        generate_rewards();
    }

    #[only_owner]
    #[endpoint(addAddressToSiblingWhitelist)]
    fn add_address_to_sibling_whitelist(&self, address: ManagedAddress) {
        self.sibling_whitelist().insert(address);
    }

    #[only_owner]
    #[endpoint(removeAddressFromSiblingWhitelist)]
    fn remove_address_from_sibling_whitelist(&self, address: ManagedAddress) {
        self.sibling_whitelist().remove(&address);
        self.sibling_supplies().remove(&address);
    }

    #[endpoint(isSiblingWhitelisted)]
    fn is_sibling_whitelisted(&self, address: ManagedAddress) -> bool {
        self.sibling_whitelist().contains(&address)
    }

    fn require_sibling_whitelisted(&self, address: ManagedAddress) {
        require!(
            self.is_sibling_whitelisted(address),
            ERROR_SIBLING_PERMISSIONS
        )
    }

    #[view(getSiblingWhitelist)]
    #[storage_mapper("sibling_whitelist")]
    fn sibling_whitelist(&self) -> SetMapper<ManagedAddress>;

    #[storage_mapper("sibling_supplies")]
    fn sibling_supplies(&self) -> MapMapper<ManagedAddress, BigUint>;
}
