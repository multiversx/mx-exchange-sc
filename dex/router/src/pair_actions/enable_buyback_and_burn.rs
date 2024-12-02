use pair::fee::endpoints::ProxyTrait as _;

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait EnableBuybackAndBurnModule:
    crate::config::ConfigModule
    + pair::read_pair_storage::ReadPairStorageModule
    + crate::views::ViewsModule
{
    #[only_owner]
    #[endpoint(setTokenToBuy)]
    fn set_token_to_buy(&self, token_to_buy: TokenIdentifier) {
        require!(
            token_to_buy.is_valid_esdt_identifier(),
            "Invalid token to buy"
        );

        self.token_to_buy().set(token_to_buy);
    }

    fn enable_buyback_and_burn(&self, pair_address: ManagedAddress) {
        let first_token_id = self.get_first_token_id_mapper(pair_address.clone()).get();
        let second_token_id = self.get_second_token_id_mapper(pair_address.clone()).get();
        let common_tokens_mapper = self.common_tokens_for_user_pairs();
        let common_token_id = if common_tokens_mapper.contains(&first_token_id) {
            first_token_id
        } else if common_tokens_mapper.contains(&second_token_id) {
            second_token_id
        } else {
            return;
        };

        let token_to_buy = self.token_to_buy().get();
        let found_pair = self.get_pair(token_to_buy.clone(), common_token_id);
        if found_pair.is_zero() {
            return;
        }

        self.whitelist_in_found_pair(found_pair.clone(), pair_address.clone());
        self.add_trusted_swap_current_pair(found_pair.clone(), pair_address.clone());
        self.set_fee_on_pair(pair_address, token_to_buy);
    }

    fn whitelist_in_found_pair(&self, found_pair: ManagedAddress, current_pair: ManagedAddress) {
        self.pair_contract_proxy_buyback(found_pair)
            .whitelist_endpoint(current_pair)
            .execute_on_dest_context()
    }

    fn add_trusted_swap_current_pair(
        &self,
        found_pair: ManagedAddress,
        current_pair: ManagedAddress,
    ) {
        let first_token_id_found_pair = self.get_first_token_id_mapper(found_pair.clone()).get();
        let second_token_id_found_pair = self.get_second_token_id_mapper(found_pair.clone()).get();
        self.pair_contract_proxy_buyback(current_pair)
            .add_trusted_swap_pair(
                found_pair,
                first_token_id_found_pair,
                second_token_id_found_pair,
            )
            .execute_on_dest_context()
    }

    fn set_fee_on_pair(&self, current_pair: ManagedAddress, fee_token_id: TokenIdentifier) {
        self.pair_contract_proxy_buyback(current_pair)
            .set_fee_on(ManagedAddress::zero(), fee_token_id)
            .execute_on_dest_context()
    }

    #[proxy]
    fn pair_contract_proxy_buyback(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[storage_mapper("tokenToBuy")]
    fn token_to_buy(&self) -> SingleValueMapper<TokenIdentifier>;
}
