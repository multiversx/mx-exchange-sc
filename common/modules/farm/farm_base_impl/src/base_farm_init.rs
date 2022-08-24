elrond_wasm::imports!();

use common_errors::{ERROR_NOT_AN_ESDT, ERROR_SAME_TOKEN_IDS, ERROR_ZERO_AMOUNT};
use pausable::State;

#[elrond_wasm::module]
pub trait BaseFarmInitModule:
    config::ConfigModule
    + farm_token::FarmTokenModule
    + admin_whitelist::AdminWhitelistModule
    + pausable::PausableModule
    + elrond_wasm_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn base_farm_init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        mut admins: MultiValueEncoded<ManagedAddress>,
    ) {
        require!(
            reward_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(
            farming_token_id.is_valid_esdt_identifier(),
            ERROR_NOT_AN_ESDT
        );
        require!(division_safety_constant != 0u64, ERROR_ZERO_AMOUNT);

        let farm_token = self.farm_token().get_token_id();
        require!(reward_token_id != farm_token, ERROR_SAME_TOKEN_IDS);
        require!(farming_token_id != farm_token, ERROR_SAME_TOKEN_IDS);

        self.state().set(State::Inactive);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        self.reward_token_id().set(&reward_token_id);
        self.farming_token_id().set(&farming_token_id);

        let caller = self.blockchain().get_caller();
        self.pause_whitelist().add(&caller);

        if admins.is_empty() {
            admins.push(caller);
        }
        self.add_admins(admins);
    }
}
