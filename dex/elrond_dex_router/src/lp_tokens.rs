elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use super::factory;
use super::pair_manager;
use super::state;

const LP_TOKEN_DECIMALS: usize = 18;
const LP_TOKEN_INITIAL_SUPPLY: u64 = 1000;

#[elrond_wasm::module]
pub trait LpTokensModule:
    pair_manager::PairManagerModule
    + state::StateModule
    + factory::FactoryModule
    + token_send::TokenSendModule
{
    #[payable("EGLD")]
    #[endpoint(issueLpToken)]
    fn issue_lp_token(
        &self,
        pair_address: Address,
        tp_token_display_name: BoxedBytes,
        tp_token_ticker: BoxedBytes,
        #[payment_amount] issue_cost: Self::BigUint,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        let caller = self.blockchain().get_caller();
        if caller != self.owner().get() {
            require!(
                self.pair_creation_enabled().get(),
                "Pair creation is disabled"
            );
        }
        self.check_is_pair_sc(&pair_address)?;
        let result = self.get_pair_temporary_owner(&pair_address);

        match result {
            None => {}
            Some(temporary_owner) => {
                require!(caller == temporary_owner, "Temporary owner differs");
            }
        };

        let result = self.get_lp_token_for_pair(&pair_address);
        require!(result.is_egld(), "LP Token already issued");

        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .issue_fungible(
                issue_cost,
                &tp_token_display_name,
                &tp_token_ticker,
                &Self::BigUint::from(LP_TOKEN_INITIAL_SUPPLY),
                FungibleTokenProperties {
                    num_decimals: LP_TOKEN_DECIMALS,
                    can_freeze: true,
                    can_wipe: true,
                    can_pause: true,
                    can_mint: true,
                    can_burn: true,
                    can_change_owner: true,
                    can_upgrade: true,
                    can_add_special_roles: true,
                },
            )
            .async_call()
            .with_callback(
                self.callbacks()
                    .lp_token_issue_callback(&caller, &pair_address),
            ))
    }

    #[endpoint(setLocalRoles)]
    fn set_local_roles(&self, pair_address: Address) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        self.check_is_pair_sc(&pair_address)?;

        let pair_token = self.get_lp_token_for_pair(&pair_address);
        require!(pair_token.is_esdt(), "LP token not issued");

        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .set_special_roles(
                &pair_address,
                &pair_token,
                &[EsdtLocalRole::Mint, EsdtLocalRole::Burn],
            )
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    #[only_owner]
    #[endpoint(setLocalRolesOwner)]
    fn set_local_roles_owner(
        &self,
        token: TokenIdentifier,
        address: Address,
        #[var_args] roles: VarArgs<EsdtLocalRole>,
    ) -> SCResult<AsyncCall<Self::SendApi>> {
        require!(self.is_active(), "Not active");
        require!(!roles.is_empty(), "Empty roles");
        Ok(ESDTSystemSmartContractProxy::new_proxy_obj(self.send())
            .set_special_roles(&address, &token, roles.as_slice())
            .async_call()
            .with_callback(self.callbacks().change_roles_callback()))
    }

    #[callback]
    fn lp_token_issue_callback(
        &self,
        caller: &Address,
        address: &Address,
        #[payment_token] token_id: TokenIdentifier,
        #[payment_amount] returned_tokens: Self::BigUint,
        #[call_result] result: AsyncCallResult<()>,
    ) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.pair_temporary_owner().remove(address);
                self.set_lp_token_for_pair(address, &token_id);
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);

                if token_id.is_egld() && returned_tokens > 0 {
                    let _ = self.send().direct_egld(caller, &returned_tokens, &[]);
                }
            }
        }
    }

    #[callback]
    fn change_roles_callback(&self, #[call_result] result: AsyncCallResult<()>) {
        match result {
            AsyncCallResult::Ok(()) => {
                self.last_error_message().clear();
            }
            AsyncCallResult::Err(message) => {
                self.last_error_message().set(&message.err_msg);
            }
        }
    }
}
