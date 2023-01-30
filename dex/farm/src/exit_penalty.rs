multiversx_sc::imports!();

use common_errors::ERROR_PARAMETERS;
use common_structs::Epoch;

pub const MAX_PERCENT: u64 = 10_000;
pub const DEFAULT_PENALTY_PERCENT: u64 = 100;
pub const DEFAULT_MINUMUM_FARMING_EPOCHS: u64 = 3;
pub const DEFAULT_BURN_GAS_LIMIT: u64 = 50_000_000;
pub const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;
pub const MAX_MINIMUM_FARMING_EPOCHS: u64 = 30;

#[multiversx_sc::module]
pub trait ExitPenaltyModule: permissions_module::PermissionsModule {
    #[only_owner]
    #[endpoint]
    fn set_penalty_percent(&self, percent: u64) {
        require!(percent < MAX_PERCENT, ERROR_PARAMETERS);
        self.penalty_percent().set(percent);
    }

    #[endpoint]
    fn set_minimum_farming_epochs(&self, epochs: Epoch) {
        self.require_caller_has_admin_permissions();
        require!(epochs <= MAX_MINIMUM_FARMING_EPOCHS, ERROR_PARAMETERS);

        self.minimum_farming_epochs().set(epochs);
    }

    #[only_owner]
    #[endpoint]
    fn set_burn_gas_limit(&self, gas_limit: u64) {
        self.burn_gas_limit().set(gas_limit);
    }

    fn burn_farming_tokens(
        &self,
        farming_amount: &BigUint,
        farming_token_id: &TokenIdentifier,
        reward_token_id: &TokenIdentifier,
    ) {
        let pair_contract_address = self.pair_contract_address().get();
        if pair_contract_address.is_zero() {
            self.send()
                .esdt_local_burn(farming_token_id, 0, farming_amount);
        } else {
            let gas_limit = self.burn_gas_limit().get();
            self.pair_contract_proxy(pair_contract_address)
                .remove_liquidity_and_burn_token(reward_token_id.clone())
                .with_esdt_transfer((farming_token_id.clone(), 0, farming_amount.clone()))
                .with_gas_limit(gas_limit)
                .transfer_execute();
        }
    }

    #[proxy]
    fn pair_contract_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[view(getPenaltyPercent)]
    #[storage_mapper("penalty_percent")]
    fn penalty_percent(&self) -> SingleValueMapper<u64>;

    #[view(getMinimumFarmingEpoch)]
    #[storage_mapper("minimum_farming_epochs")]
    fn minimum_farming_epochs(&self) -> SingleValueMapper<Epoch>;

    #[view(getBurnGasLimit)]
    #[storage_mapper("burn_gas_limit")]
    fn burn_gas_limit(&self) -> SingleValueMapper<u64>;

    #[view(getPairContractManagedAddress)]
    #[storage_mapper("pair_contract_address")]
    fn pair_contract_address(&self) -> SingleValueMapper<ManagedAddress>;
}
