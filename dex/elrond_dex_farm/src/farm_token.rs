elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{FarmTokenAttributes, GenericTokenAmountPair, Nonce};

use super::config;

#[derive(Clone)]
pub struct FarmToken<M: ManagedTypeApi> {
    pub token_amount: GenericTokenAmountPair<M>,
    pub attributes: FarmTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait FarmTokenModule:
    config::ConfigModule + token_send::TokenSendModule + token_supply::TokenSupplyModule
{
    #[payable("EGLD")]
    #[endpoint(issueFarmToken)]
    fn issue_farm_token(
        &self,
        #[payment_amount] issue_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) -> SCResult<AsyncCall> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(self.farm_token_id().is_empty(), "Already issued");

        Ok(self.issue_token(issue_cost, token_display_name, token_ticker))
    }

    fn issue_token(
        &self,
        issue_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
    ) -> AsyncCall {
        self.send()
            .esdt_system_sc_proxy()
            .issue_semi_fungible(
                issue_cost,
                &token_display_name,
                &token_ticker,
                SemiFungibleTokenProperties {
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(
                self.callbacks()
                    .issue_callback(&self.blockchain().get_caller()),
            )
    }

    #[callback]
    fn issue_callback(
        &self,
        caller: &ManagedAddress,
        #[call_result] result: ManagedAsyncCallResult<TokenIdentifier>,
    ) {
        match result {
            ManagedAsyncCallResult::Ok(token_id) => {
                self.last_error_message().clear();

                if self.farm_token_id().is_empty() {
                    self.farm_token_id().set(&token_id);
                }
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                let (returned_tokens, token_id) = self.call_value().payment_token_pair();
                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) -> SCResult<AsyncCall> {
        require!(self.is_active(), "Not active");
        self.require_permissions()?;
        require!(!self.farm_token_id().is_empty(), "No farm token issued");

        let token = self.farm_token_id().get();
        Ok(self.set_local_roles(token))
    }

    fn set_local_roles(&self, token: TokenIdentifier) -> AsyncCall {
        let roles = [
            EsdtLocalRole::NftCreate,
            EsdtLocalRole::NftAddQuantity,
            EsdtLocalRole::NftBurn,
        ];

        self.send()
            .esdt_system_sc_proxy()
            .set_special_roles(
                &self.blockchain().get_sc_address(),
                &token,
                (&roles[..]).into_iter().cloned(),
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: ManagedAsyncCallResult<()>) {
        match result {
            ManagedAsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            ManagedAsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }

    fn decode_attributes(
        &self,
        attributes_raw: &BoxedBytes,
    ) -> SCResult<FarmTokenAttributes<Self::Api>> {
        Ok(self
            .serializer()
            .top_decode_from_byte_slice::<FarmTokenAttributes<Self::Api>>(
                attributes_raw.as_slice(),
            ))
    }

    fn get_farm_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> SCResult<FarmTokenAttributes<Self::Api>> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        Ok(self
            .serializer()
            .top_decode_from_managed_buffer::<FarmTokenAttributes<Self::Api>>(
                &token_info.attributes,
            ))
    }

    fn create_farm_tokens(
        &self,
        farm_amount: &BigUint,
        farm_token_id: &TokenIdentifier,
        attributes: &FarmTokenAttributes<Self::Api>,
    ) -> Nonce {
        self.nft_create_tokens(farm_token_id, farm_amount, attributes);
        self.increase_nonce()
    }

    fn burn_farm_tokens_from_payments(
        &self,
        payments: &[EsdtTokenPayment<Self::Api>],
    ) -> SCResult<()> {
        for entry in payments {
            self.burn_farm_tokens(&entry.token_identifier, entry.token_nonce, &entry.amount)?;
        }
        Ok(())
    }

    fn burn_farm_tokens(
        &self,
        farm_token_id: &TokenIdentifier,
        farm_token_nonce: Nonce,
        amount: &BigUint,
    ) -> SCResult<()> {
        let farm_amount = self.get_farm_token_supply();
        require!(&farm_amount >= amount, "Not enough supply");
        self.nft_burn_tokens(farm_token_id, farm_token_nonce, amount);
        Ok(())
    }

    fn increase_nonce(&self) -> Nonce {
        let new_nonce = self.farm_token_nonce().get() + 1;
        self.farm_token_nonce().set(&new_nonce);
        new_nonce
    }
}
