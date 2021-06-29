#![allow(non_snake_case)]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{Nonce};

const MAX_FUNDS_ENTRIES: usize = 10;

#[elrond_wasm_derive::module]
pub trait ProxyCommonModule {
    fn require_permissions(&self) -> SCResult<()> {
        only_owner!(self, "Permission denied");
        Ok(())
    }

    #[payable("*")]
    #[endpoint]
    fn acceptPay(
        &self,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] amount: Self::BigUint,
        #[payment_nonce] token_nonce: Nonce,
    ) {
        if amount == 0 {
            return;
        }

        if self.current_tx_accepted_funds().len() > MAX_FUNDS_ENTRIES {
            self.current_tx_accepted_funds().clear();
        }

        let entry = self
            .current_tx_accepted_funds()
            .get(&(token_id.clone(), token_nonce));
        match entry {
            Some(value) => {
                self.current_tx_accepted_funds()
                    .insert((token_id, token_nonce), value + amount);
            }
            None => {
                self.current_tx_accepted_funds()
                    .insert((token_id, token_nonce), amount);
            }
        }
    }

    fn reset_received_funds_on_current_tx(&self) {
        self.current_tx_accepted_funds().clear();
    }

    fn validate_received_funds_chunk(
        &self,
        received_funds: Vec<(&TokenIdentifier, Nonce, &Self::BigUint)>,
    ) -> SCResult<()> {
        let big_zero = Self::BigUint::zero();

        for funds in received_funds {
            let token_id = funds.0;
            let nonce = funds.1;
            let amount = funds.2;

            if amount == &big_zero {
                continue;
            }

            self.validate_received_funds_on_current_tx(token_id, nonce, amount)?;
            let old_amount = self
                .current_tx_accepted_funds()
                .get(&(token_id.clone(), nonce))
                .unwrap();

            if &old_amount == amount {
                self.current_tx_accepted_funds()
                    .remove(&(token_id.clone(), nonce));
            } else {
                self.current_tx_accepted_funds()
                    .insert((token_id.clone(), nonce), &old_amount - amount);
            }
        }

        require!(
            self.current_tx_accepted_funds().is_empty(),
            "More funds were received"
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

    fn direct_generic(
        &self,
        to: &Address,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
    ) {
        if nonce == 0 {
            let _ =
                self.send()
                    .direct_esdt_execute(to, token_id, amount, 0, &[], &ArgBuffer::new());
        } else {
            self.send().direct_nft(to, token_id, nonce, amount, &[]);
        }
    }

    fn direct_generic_safe(
        &self,
        to: &Address,
        token_id: &TokenIdentifier,
        nonce: Nonce,
        amount: &Self::BigUint,
    ) {
        if amount > &0 {
            self.direct_generic(to, token_id, nonce, amount);
        }
    }

    #[storage_mapper("current_tx_accepted_funds")]
    fn current_tx_accepted_funds(
        &self,
    ) -> MapMapper<Self::Storage, (TokenIdentifier, Nonce), Self::BigUint>;

    #[view(getAssetTokenId)]
    #[storage_mapper("asset_token_id")]
    fn asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;

    #[view(getLockedAssetTokenId)]
    #[storage_mapper("locked_asset_token_id")]
    fn locked_asset_token_id(&self) -> SingleValueMapper<Self::Storage, TokenIdentifier>;
}
