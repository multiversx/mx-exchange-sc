elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait ConfigModule {
    #[only_owner]
    #[endpoint(addKnownPairContracts)]
    fn add_known_pair_contracts(&self, pair_contracts: MultiValueEncoded<ManagedAddress>) {
        let mut mapper = self.known_pair_contracts();
        for sc in pair_contracts {
            require!(
                self.blockchain().is_smart_contract(&sc),
                "Invalid pair SC address"
            );
            let _ = mapper.insert(sc);
        }
    }

    #[only_owner]
    #[endpoint(removeKnownPairContracts)]
    fn remove_known_pair_contracts(&self, pair_contracts: MultiValueEncoded<ManagedAddress>) {
        let mut mapper = self.known_pair_contracts();
        for sc in pair_contracts {
            let _ = mapper.swap_remove(&sc);
        }
    }

    #[only_owner]
    #[endpoint(addKnownTokens)]
    fn add_known_tokens(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut all_tokens_vec = self.all_tokens().get();
        let known_tokens_mapper = self.known_tokens();
        for token in tokens {
            require!(token.is_valid_esdt_identifier(), "Invalid token ID");

            if !known_tokens_mapper.contains(&token) {
                known_tokens_mapper.add(&token);
                all_tokens_vec.push(token);
            }
        }

        self.all_tokens().set(&all_tokens_vec);
    }

    #[only_owner]
    #[endpoint(removeKnownTokens)]
    fn remove_known_tokens(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut all_tokens_vec = self.all_tokens().get();
        let known_tokens_mapper = self.known_tokens();
        for token in tokens {
            require!(token.is_valid_esdt_identifier(), "Invalid token ID");

            if known_tokens_mapper.contains(&token) {
                known_tokens_mapper.remove(&token);

                unsafe {
                    let index = all_tokens_vec.find(&token).unwrap_unchecked();
                    all_tokens_vec.remove(index);
                }
            }
        }

        self.all_tokens().set(&all_tokens_vec);
    }

    #[view(getAllTokens)]
    fn get_all_tokens(&self) -> MultiValueEncoded<TokenIdentifier> {
        self.all_tokens().get().into()
    }

    #[view(getAllKnownPairContracts)]
    #[storage_mapper("knownPairContracts")]
    fn known_pair_contracts(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("knownTokens")]
    fn known_tokens(&self) -> WhitelistMapper<Self::Api, TokenIdentifier>;

    #[storage_mapper("allTokens")]
    fn all_tokens(&self) -> SingleValueMapper<ManagedVec<TokenIdentifier>>;
}
