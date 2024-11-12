multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait UnlockedTokenTransferModule:
    utils::UtilsModule
    + multiversx_sc_modules::pause::PauseModule
    + crate::token_whitelist::TokenWhitelistModule
{
    #[only_owner]
    #[endpoint(addToUnlockedTokenTransferWhitelist)]
    fn add_to_unlocked_token_transfer_whitelist(
        &self,
        sc_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        let mapper = self.unlocked_token_transfer_whitelist();
        for sc_addr in sc_addresses {
            self.require_sc_address(&sc_addr);

            mapper.add(&sc_addr);
        }
    }

    #[only_owner]
    #[endpoint(removeFromUnlockedTokenTransferWhitelist)]
    fn remove_from_unlocked_token_transfer_whitelist(
        &self,
        sc_addresses: MultiValueEncoded<ManagedAddress>,
    ) {
        let mapper = self.unlocked_token_transfer_whitelist();
        for sc_addr in sc_addresses {
            mapper.remove(&sc_addr);
        }
    }

    #[endpoint(transferUnlockedToken)]
    fn transfer_unlocked_token(&self, dest: ManagedAddress, amount: BigUint) {
        self.require_not_paused();
        require!(amount != 0, "Invalid amount");

        let caller = self.blockchain().get_caller();
        require!(
            self.unlocked_token_transfer_whitelist().contains(&caller),
            "May not call this endpoint"
        );

        let base_asset_token_id = self.base_asset_token_id().get();
        self.send()
            .esdt_local_mint(&base_asset_token_id, 0, &amount);
        self.send()
            .direct_esdt(&dest, &base_asset_token_id, 0, &amount);
    }

    #[storage_mapper("ulkTokenTransfWhitelist")]
    fn unlocked_token_transfer_whitelist(&self) -> WhitelistMapper<ManagedAddress>;
}
