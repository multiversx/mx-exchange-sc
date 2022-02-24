#![no_std]

elrond_wasm::imports!();

#[elrond_wasm::contract]
pub trait FarmV12Mock {
    #[init]
    fn init(
        &self,
        _router_address: ManagedAddress,
        reward_token_id: TokenIdentifier,
        _farming_token_id: TokenIdentifier,
        _locked_asset_factory_address: ManagedAddress,
        _division_safety_constant: BigUint,
        _pair_contract_address: ManagedAddress,
    ) -> SCResult<()> {
        self.reward_token_id().set(&reward_token_id);
        Ok(())
    }

    #[payable("*")]
    #[endpoint(migrateToNewFarm)]
    fn migrate_to_new_farm(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] token_nonce: u64,
        #[payment_amount] amount: BigUint,
        _orig_caller: ManagedAddress,
    ) -> SCResult<MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>>> {
        let payment_1 = EsdtTokenPayment::new(payment_token_id, token_nonce, amount);
        let payment_2 = EsdtTokenPayment::new(self.reward_token_id().get(), 0, BigUint::zero());

        Ok(MultiValue2::from((payment_1, payment_2)))
    }

    #[storage_mapper("reward_token_id")]
    fn reward_token_id(&self) -> SingleValueMapper<TokenIdentifier>;
}
