#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

#[elrond_wasm::module]
pub trait TokenSupplyModule {
    fn nft_create_tokens<T: elrond_codec::TopEncode>(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &T,
    ) -> u64 {
        self.increase_generated_amount(token_id, amount);
        let mut uris = ManagedVec::new();
        uris.push(self.types().managed_buffer_new());
        self.send().esdt_nft_create::<T>(
            token_id,
            amount,
            &self.types().managed_buffer_new(),
            &BigUint::zero(),
            &self.types().managed_buffer_new(),
            attributes,
            &uris,
        )
    }

    fn nft_add_quantity_tokens(&self, token_id: &TokenIdentifier, nonce: Nonce, amount: &BigUint) {
        self.increase_generated_amount(token_id, amount);
        self.send().esdt_local_mint(token_id, nonce, amount);
    }

    fn nft_burn_tokens(&self, token_id: &TokenIdentifier, nonce: Nonce, amount: &BigUint) {
        self.increase_burned_amount(token_id, amount);
        self.send().esdt_local_burn(token_id, nonce, amount);
    }

    fn mint_tokens(&self, token_id: &TokenIdentifier, amount: &BigUint) {
        self.increase_generated_amount(token_id, amount);
        self.send().esdt_local_mint(token_id, 0, amount);
    }

    fn burn_tokens(&self, token_id: &TokenIdentifier, amount: &BigUint) {
        self.increase_burned_amount(token_id, amount);
        self.send().esdt_local_burn(token_id, 0, amount);
    }

    fn increase_generated_amount(&self, token_id: &TokenIdentifier, amount: &BigUint) {
        let old_amount = self.get_generated_token_amount(token_id);
        self.generated_tokens()
            .insert(token_id.clone(), &old_amount + amount);
    }

    fn increase_burned_amount(&self, token_id: &TokenIdentifier, amount: &BigUint) {
        let old_amount = self.get_burned_token_amount(token_id);
        self.burned_tokens()
            .insert(token_id.clone(), &old_amount + amount);
    }

    fn get_total_supply(&self, token_id: &TokenIdentifier) -> SCResult<BigUint> {
        let generated_amount = self.get_generated_token_amount(token_id);
        let burned_amount = self.get_burned_token_amount(token_id);
        require!(generated_amount >= burned_amount, "Negative total supply");
        Ok(generated_amount - burned_amount)
    }

    #[view(getGeneratedTokenAmountList)]
    fn get_generated_token_amount_list(&self) -> ManagedMultiResultVec<(TokenIdentifier, BigUint)> {
        let mut result = ManagedMultiResultVec::new(self.type_manager());
        for item in self.generated_tokens().iter() {
            result.push(item)
        }
        result
    }

    #[view(getBurnedTokenAmountList)]
    fn get_burned_token_amount_list(&self) -> ManagedMultiResultVec<(TokenIdentifier, BigUint)> {
        let mut result = ManagedMultiResultVec::new(self.type_manager());
        for item in self.burned_tokens().iter() {
            result.push(item)
        }
        result
    }

    #[view(getGeneratedTokenAmount)]
    fn get_generated_token_amount(&self, token_id: &TokenIdentifier) -> BigUint {
        self.generated_tokens()
            .get(token_id)
            .unwrap_or(BigUint::zero())
    }

    #[view(getBurnedTokenAmount)]
    fn get_burned_token_amount(&self, token_id: &TokenIdentifier) -> BigUint {
        self.burned_tokens()
            .get(token_id)
            .unwrap_or(BigUint::zero())
    }

    #[storage_mapper("generated_tokens")]
    fn generated_tokens(&self) -> MapMapper<TokenIdentifier, BigUint>;

    #[storage_mapper("burned_tokens")]
    fn burned_tokens(&self) -> MapMapper<TokenIdentifier, BigUint>;
}
