multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UnlockedTokenTransferModule:
    utils::UtilsModule
    + multiversx_sc_modules::pause::PauseModule
    + crate::token_whitelist::TokenWhitelistModule
{
    #[only_owner]
    #[endpoint(addToUnlockedTokenMintWhitelist)]
    fn add_to_unlocked_token_mint_whitelist(
        &self,
        sc_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        let mut mapper = self.unlocked_token_mint_whitelist();
        for sc_addr in sc_addresses {
            self.require_sc_address(&sc_addr);

            let _ = mapper.insert(sc_addr);
        }
    }

    #[only_owner]
    #[endpoint(removeFromUnlockedTokenMintWhitelist)]
    fn remove_from_unlocked_token_mint_whitelist(
        &self,
        sc_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        let mut mapper = self.unlocked_token_mint_whitelist();
        for sc_addr in sc_addresses {
            let _ = mapper.swap_remove(&sc_addr);
        }
    }

    #[only_owner]
    #[endpoint(setMultisigAddress)]
    fn set_multisig_address(&self, multisig_address: ManagedAddress) {
        self.multisig_address().set(multisig_address);
    }

    #[endpoint(transferUnlockedToken)]
    fn transfer_unlocked_token(&self, amount: BigUint) {
        self.require_not_paused();
        require!(amount != 0, "Invalid amount");

        let caller = self.blockchain().get_caller();
        require!(
            self.unlocked_token_mint_whitelist().contains(&caller),
            "Address is not whitelisted for token transfer"
        );
        require!(
            !self.multisig_address().is_empty(),
            "No multisig address set"
        );

        let multisig_address = self.multisig_address().get();
        let base_asset_token_id = self.base_asset_token_id().get();
        self.send()
            .esdt_local_mint(&base_asset_token_id, 0, &amount);
        self.send()
            .direct_esdt(&multisig_address, &base_asset_token_id, 0, &amount);
    }

    #[storage_mapper("multisigAddress")]
    fn multisig_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[storage_mapper("unlockedTokenMintWhitelist")]
    fn unlocked_token_mint_whitelist(&self) -> UnorderedSetMapper<ManagedAddress>;
}
