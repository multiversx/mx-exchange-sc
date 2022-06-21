#![no_std]

use common_errors::ERROR_SIBLING_PERMISSIONS;

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

#[elrond_wasm::module]
pub trait MultishardModule: config::ConfigModule + token_send::TokenSendModule {
    fn synchronize_impl<F: FnOnce()>(&self, generate_rewards: F) {
        let farm_token_supply = self.farm_token_supply().get();
        for sibling in self.sibling_whitelist().iter() {
            self.proxy_call_accept_synchronization(sibling, &farm_token_supply);
        }
        self.update_sibling_supply(&self.blockchain().get_sc_address(), farm_token_supply);

        self.try_create_checkpoint(generate_rewards);
    }

    fn proxy_call_accept_synchronization(
        &self,
        sibling: ManagedAddress,
        farm_token_supply: &BigUint,
    ) {
        let endpoint_name = ManagedBuffer::new_from_bytes(b"acceptSynchronization");
        let mut accept_sync: ContractCall<<Self as ContractBase>::Api, ()> =
            self.send().contract_call(sibling, endpoint_name);
        accept_sync.push_endpoint_arg(&farm_token_supply);
        accept_sync.transfer_execute();
    }

    fn accept_synchronization_impl<F: FnOnce()>(
        &self,
        sibling_supply: BigUint,
        generate_rewards: F,
    ) {
        let sibling = self.blockchain().get_caller();
        self.require_sibling_whitelisted(&sibling);
        self.update_sibling_supply(&sibling, sibling_supply);
        self.try_create_checkpoint(generate_rewards);
    }

    fn try_create_checkpoint<F: FnOnce()>(&self, generate_rewards: F) {
        if self.sibling_supplies_received().get() <= self.sibling_whitelist().len() {
            return;
        }

        let mut global_liquidity = BigUint::zero();
        for sibling in self.sibling_whitelist().iter() {
            global_liquidity += self.get_sibling_supply(&sibling);
            self.sibling_supply(&sibling).clear();
        }

        let sc_address = self.blockchain().get_sc_address();
        let local_farm_token_supply = self.get_sibling_supply(&sc_address);
        global_liquidity += &local_farm_token_supply;
        self.sibling_supply(&sc_address).clear();

        self.sibling_supplies_received().clear();

        let current_nonce = self.blockchain().get_block_nonce();
        self.current_checkpoint_block_nonce().set(current_nonce);
        self.local_farm_token_supply().set(local_farm_token_supply);
        self.global_farm_token_supply().set(global_liquidity);
        let total_sibling_count = self.sibling_whitelist().len() + 1;
        self.default_ratio()
            .set(&BigUint::from(total_sibling_count));

        generate_rewards();
    }

    #[only_owner]
    #[endpoint(setSiblingWhitelist)]
    fn set_sibling_whitelist(&self, sibling_addresses: MultiValueEncoded<ManagedAddress>) {
        for sibling in self.sibling_whitelist().iter() {
            self.sibling_supply(&sibling).clear();
        }
        self.sibling_supply(&self.blockchain().get_sc_address())
            .clear();
        self.sibling_supplies_received().clear();
        self.sibling_whitelist().clear();

        for address in sibling_addresses {
            self.sibling_whitelist().insert(address);
        }
        self.sibling_whitelist()
            .swap_remove(&self.blockchain().get_sc_address());
    }

    fn get_sibling_supply(&self, sibling: &ManagedAddress) -> BigUint {
        self.sibling_supply(sibling)
            .get()
            .unwrap_or_else(|| sc_panic!("Expected supply"))
    }

    fn update_sibling_supply(&self, sibling: &ManagedAddress, new_supply: BigUint) {
        self.sibling_supply(sibling).update(|supply| {
            if *supply == Option::None {
                self.sibling_supplies_received()
                    .update(|received| *received += 1);
            }
            *supply = Some(new_supply)
        });
    }

    #[endpoint(isSiblingWhitelisted)]
    fn is_sibling_whitelisted(&self, address: &ManagedAddress) -> bool {
        self.sibling_whitelist().contains(address)
    }

    fn require_sibling_whitelisted(&self, address: &ManagedAddress) {
        require!(
            self.is_sibling_whitelisted(address),
            ERROR_SIBLING_PERMISSIONS
        )
    }

    #[view(getSiblingWhitelist)]
    #[storage_mapper("sibling_whitelist")]
    fn sibling_whitelist(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[view(getSiblingSupply)]
    #[storage_mapper("sibling_supply")]
    fn sibling_supply(&self, sibling: &ManagedAddress) -> SingleValueMapper<Option<BigUint>>;

    #[view(getSiblingSuppliesReceived)]
    #[storage_mapper("sibling_supplies_received")]
    fn sibling_supplies_received(&self) -> SingleValueMapper<usize>;
}
