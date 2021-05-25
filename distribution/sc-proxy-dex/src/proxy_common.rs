#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;

#[elrond_wasm_derive::module]
pub trait ProxyCommonModule {
    #[endpoint(addAcceptedLockedAssetTokenId)]
    fn add_accepted_locked_asset_token_id(&self, token_id: TokenIdentifier) -> SCResult<()> {
        self.require_permissions()?;
        self.accepted_locked_assets().insert(token_id);
        Ok(())
    }

    #[endpoint(removeAcceptedLockedAssetTokenId)]
    fn remove_accepted_locked_asset_token_id(&self, token_id: TokenIdentifier) -> SCResult<()> {
        self.require_permissions()?;
        self.require_is_accepted_locked_asset(&token_id)?;
        self.accepted_locked_assets().remove(&token_id);
        Ok(())
    }

    fn require_is_accepted_locked_asset(&self, token_id: &TokenIdentifier) -> SCResult<()> {
        require!(
            self.accepted_locked_assets().contains(token_id),
            "Not an accepted locked asset"
        );
        Ok(())
    }

    fn require_permissions(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn acceptPay(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment] amount: Self::BigUint,
    ) {
        let tx_hash = self.blockchain().get_tx_hash();

        if !self.last_tx_hash().is_empty() {
            let last_tx_hash = self.last_tx_hash().get();
            if tx_hash != last_tx_hash {
                self.last_tx_hash().set(&tx_hash);
                self.last_tx_accepted_funds().clear();
            }
        } else {
            self.last_tx_hash().set(&tx_hash);
        }

        let token_nonce = self.call_value().esdt_token_nonce();
        self.last_tx_accepted_funds()
            .insert((token_id, token_nonce), amount);
    }

    fn validate_received_funds_on_current_tx(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        amount: &Self::BigUint,
    ) -> SCResult<()> {
        if self.last_tx_hash().is_empty() {
            return sc_error!("No funds received");
        }
        if amount == &Self::BigUint::zero() {
            return Ok(());
        }

        let tx_hash = self.blockchain().get_tx_hash();
        let last_tx_hash = self.last_tx_hash().get();

        if tx_hash == last_tx_hash {
            let result = self
                .last_tx_accepted_funds()
                .get(&(token_id.clone(), token_nonce));

            match result {
                Some(available_amount) => {
                    if &available_amount >= amount {
                        Ok(())
                    } else {
                        sc_error!("Available amount is not enough")
                    }
                }
                None => {
                    sc_error!("No available funds of this type")
                }
            }
        } else {
            sc_error!("No available funds for this tx hash")
        }
    }

    #[storage_mapper("last_tx_hash")]
    fn last_tx_hash(&self) -> SingleValueMapper<Self::Storage, H256>;

    #[storage_mapper("last_tx_accepted_funds")]
    fn last_tx_accepted_funds(
        &self,
    ) -> MapMapper<Self::Storage, (TokenIdentifier, Nonce), Self::BigUint>;

    #[view(getAcceptedLockedAssetsTokenIds)]
    #[storage_mapper("accepted_locked_assets")]
    fn accepted_locked_assets(&self) -> SetMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("distributed_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
