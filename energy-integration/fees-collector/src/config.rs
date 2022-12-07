use unwrappable::Unwrappable;

elrond_wasm::imports!();

#[elrond_wasm::module]
pub trait ConfigModule: utils::UtilsModule {
    #[only_owner]
    #[endpoint(addKnownContracts)]
    fn add_known_contracts(&self, contracts: MultiValueEncoded<ManagedAddress>) {
        let mut mapper = self.known_contracts();
        for sc in contracts {
            self.require_sc_address(&sc);
            let _ = mapper.insert(sc);
        }
    }

    #[only_owner]
    #[endpoint(removeKnownContracts)]
    fn remove_known_contracts(&self, contracts: MultiValueEncoded<ManagedAddress>) {
        let mut mapper = self.known_contracts();
        for sc in contracts {
            let _ = mapper.swap_remove(&sc);
        }
    }

    #[only_owner]
    #[endpoint(addKnownTokens)]
    fn add_known_tokens(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut all_tokens_vec = self.all_tokens().get();
        let known_tokens_mapper = self.known_tokens();
        for token in tokens {
            self.require_valid_token_id(&token);

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
        let empty_token_id = TokenIdentifier::from(ManagedBuffer::new());
        for token in tokens {
            if known_tokens_mapper.contains(&token) {
                let index = all_tokens_vec.find(&token).unwrap_or_panic::<Self::Api>();
                all_tokens_vec
                    .set(index, &empty_token_id)
                    .unwrap_or_panic::<Self::Api>();

                known_tokens_mapper.remove(&token);
            }
        }

        self.all_tokens().set(&all_tokens_vec);
    }

    #[view(getLockedTokenId)]
    #[storage_mapper("lockedTokenId")]
    fn locked_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getAllTokens)]
    fn get_all_tokens(&self) -> MultiValueEncoded<TokenIdentifier> {
        self.all_tokens().get().into()
    }

    #[view(getAllKnownContracts)]
    #[storage_mapper("knownContracts")]
    fn known_contracts(&self) -> UnorderedSetMapper<ManagedAddress>;

    #[storage_mapper("knownTokens")]
    fn known_tokens(&self) -> WhitelistMapper<Self::Api, TokenIdentifier>;

    #[storage_mapper("allTokens")]
    fn all_tokens(&self) -> SingleValueMapper<ManagedVec<TokenIdentifier>>;
}
