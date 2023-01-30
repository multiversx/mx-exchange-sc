multiversx_sc::imports!();

use common_errors::{ERROR_NOT_ACTIVE, ERROR_NO_FARM_TOKEN};
use pausable::State;

#[multiversx_sc::module]
pub trait BaseFarmValidationModule {
    fn validate_contract_state(&self, current_state: State, farm_token_id: &TokenIdentifier) {
        require!(current_state == State::Active, ERROR_NOT_ACTIVE);
        require!(
            farm_token_id.is_valid_esdt_identifier(),
            ERROR_NO_FARM_TOKEN
        );
    }
}
