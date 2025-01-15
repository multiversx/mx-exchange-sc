multiversx_sc::imports!();

use common_errors::{ERROR_NOT_AN_ESDT, ERROR_ZERO_AMOUNT};
use pausable::State;
use permissions_module::Permissions;

#[multiversx_sc::module]
pub trait BaseFarmInitModule:
    config::ConfigModule
    + farm_token::FarmTokenModule
    + permissions_module::PermissionsModule
    + pausable::PausableModule
    + multiversx_sc_modules::default_issue_callbacks::DefaultIssueCallbacksModule
{
    fn base_farm_init(
        &self,
        reward_token_id: TokenIdentifier,
        farming_token_id: TokenIdentifier,
        division_safety_constant: BigUint,
        owner: ManagedAddress,
        admins: MultiValueEncoded<ManagedAddress>,
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

        self.state().set(State::Inactive);
        self.division_safety_constant()
            .set_if_empty(&division_safety_constant);

        self.reward_token_id().set_if_empty(&reward_token_id);
        self.farming_token_id().set_if_empty(&farming_token_id);

        if !owner.is_zero() {
            self.add_permissions(owner, Permissions::OWNER | Permissions::PAUSE);
        }

        let caller = self.blockchain().get_caller();
        if admins.is_empty() {
            // backwards compatibility
            let all_permissions = Permissions::OWNER | Permissions::ADMIN | Permissions::PAUSE;
            self.add_permissions(caller, all_permissions);
        } else {
            self.add_permissions(caller, Permissions::OWNER | Permissions::PAUSE);
            self.add_permissions_for_all(admins, Permissions::ADMIN);
        };
    }
}
