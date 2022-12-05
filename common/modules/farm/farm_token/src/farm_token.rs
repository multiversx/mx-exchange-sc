#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs_old::{FarmTokenAttributes, Nonce};
use elrond_wasm::elrond_codec::TopEncode;

static SET_SPECIAL_ROLE_ENDPOINT_NAME: &[u8] = b"setSpecialRole";
static TRANSFER_ROLE_NAME: &[u8] = b"ESDTTransferRole";

#[derive(ManagedVecItem, Clone)]
pub struct FarmToken<M: ManagedTypeApi> {
    pub token_amount: EsdtTokenPayment<M>,
    pub attributes: FarmTokenAttributes<M>,
}

#[elrond_wasm::module]
pub trait FarmTokenModule: config::ConfigModule + token_send::TokenSendModule {
    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(registerFarmToken)]
    fn register_farm_token(
        &self,
        #[payment_amount] register_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        require!(self.farm_token_id().is_empty(), "Token exists already");

        self.register_token(
            register_cost,
            token_display_name,
            token_ticker,
            num_decimals,
        )
    }

    fn register_token(
        &self,
        register_cost: BigUint,
        token_display_name: ManagedBuffer,
        token_ticker: ManagedBuffer,
        num_decimals: usize,
    ) {
        self.send()
            .esdt_system_sc_proxy()
            .register_meta_esdt(
                register_cost,
                &token_display_name,
                &token_ticker,
                MetaTokenProperties {
                    num_decimals,
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
                    .register_callback(&self.blockchain().get_caller()),
            )
            .call_and_exit()
    }

    #[callback]
    fn register_callback(
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

    #[only_owner]
    #[endpoint(setLocalRolesFarmToken)]
    fn set_local_roles_farm_token(&self) {
        require!(!self.farm_token_id().is_empty(), "No farm token");

        let token = self.farm_token_id().get();
        self.set_local_roles(token)
    }

    fn set_local_roles(&self, token: TokenIdentifier) {
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
                roles.iter().cloned(),
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback())
            .call_and_exit()
    }

    #[only_owner]
    #[endpoint(setTransferRoleFarmToken)]
    fn set_transfer_role_farm_token(&self, #[var_args] opt_address: OptionalValue<ManagedAddress>) {
        let farm_token_id = self.farm_token_id().get();
        self.role_management_common(farm_token_id, SET_SPECIAL_ROLE_ENDPOINT_NAME, opt_address);
    }

    fn role_management_common(
        &self,
        locked_token_id: TokenIdentifier,
        endpoint_name: &[u8],
        opt_address: OptionalValue<ManagedAddress>,
    ) -> ! {
        let role_dest_address = self.resolve_address(opt_address);
        let esdt_system_sc_addr = self.send().esdt_system_sc_proxy().esdt_system_sc_address();
        let mut contract_call = ContractCall::<_, ()>::new(
            esdt_system_sc_addr,
            ManagedBuffer::new_from_bytes(endpoint_name),
        );
        contract_call.push_endpoint_arg(&locked_token_id);
        contract_call.push_endpoint_arg(&role_dest_address);
        contract_call.push_endpoint_arg(&TRANSFER_ROLE_NAME);

        contract_call.async_call().call_and_exit();
    }

    fn resolve_address(&self, opt_address: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_address {
            OptionalValue::Some(addr) => addr,
            OptionalValue::None => self.blockchain().get_sc_address(),
        }
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

    fn get_farm_attributes(
        &self,
        token_id: &TokenIdentifier,
        token_nonce: u64,
    ) -> FarmTokenAttributes<Self::Api> {
        let token_info = self.blockchain().get_esdt_token_data(
            &self.blockchain().get_sc_address(),
            token_id,
            token_nonce,
        );

        token_info.decode_attributes()
    }

    fn burn_farm_tokens_from_payments(&self, payments: &ManagedVec<EsdtTokenPayment<Self::Api>>) {
        let mut total_amount = BigUint::zero();
        for entry in payments.iter() {
            total_amount += &entry.amount;
            self.send()
                .esdt_local_burn(&entry.token_identifier, entry.token_nonce, &entry.amount);
        }
        self.farm_token_supply().update(|x| *x -= total_amount);
    }

    fn mint_farm_tokens<T: TopEncode>(
        &self,
        token_id: &TokenIdentifier,
        amount: &BigUint,
        attributes: &T,
    ) -> u64 {
        let new_nonce = self.nft_create_tokens(token_id, amount, attributes);
        self.farm_token_supply().update(|x| *x += amount);
        new_nonce
    }

    fn burn_farm_tokens(&self, token_id: &TokenIdentifier, nonce: Nonce, amount: &BigUint) {
        self.send().esdt_local_burn(token_id, nonce, amount);
        self.farm_token_supply().update(|x| *x -= amount);
    }
}
