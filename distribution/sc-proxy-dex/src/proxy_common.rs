#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

type Nonce = u64;
const MAX_FUNDS_EXTRIES: usize = 10;

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
        if self.current_tx_accepted_funds().len() > MAX_FUNDS_EXTRIES {
            self.current_tx_accepted_funds().clear();
        }

        let token_nonce = self.call_value().esdt_token_nonce();
        self.current_tx_accepted_funds()
            .insert((token_id, token_nonce), amount);
    }

    fn reset_received_funds_on_current_tx(&self) {
        self.current_tx_accepted_funds().clear();
    }

    fn validate_received_funds_on_current_tx_size(&self, desired_size: usize) -> SCResult<()> {
        require!(
            self.current_tx_accepted_funds().len() == desired_size,
            "Bad received funds size"
        );
        Ok(())
    }

    fn validate_received_funds_on_current_tx(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: Nonce,
        amount: &Self::BigUint,
    ) -> SCResult<()> {
        if amount == &Self::BigUint::zero() {
            return Ok(());
        }

        let result = self
            .current_tx_accepted_funds()
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
    }

    #[storage_mapper("current_tx_accepted_funds")]
    fn current_tx_accepted_funds(
        &self,
    ) -> MapMapper<Self::Storage, (TokenIdentifier, Nonce), Self::BigUint>;

    #[view(getAcceptedLockedAssetsTokenIds)]
    #[storage_mapper("accepted_locked_assets")]
    fn accepted_locked_assets(&self) -> SetMapper<Self::Storage, TokenIdentifier>;

    #[storage_mapper("distributed_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
