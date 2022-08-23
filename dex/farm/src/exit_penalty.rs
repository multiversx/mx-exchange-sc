elrond_wasm::imports!();

use common_errors::ERROR_PARAMETERS;
use common_structs::Epoch;

pub const MAX_PERCENT: u64 = 10_000;
pub const DEFAULT_PENALTY_PERCENT: u64 = 100;
pub const DEFAULT_MINUMUM_FARMING_EPOCHS: u64 = 3;
pub const DEFAULT_BURN_GAS_LIMIT: u64 = 50_000_000;
pub const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;
pub const MAX_MINIMUM_FARMING_EPOCHS: u64 = 30;

#[elrond_wasm::module]
pub trait ExitPenaltyModule: admin_whitelist::AdminWhitelistModule {
    #[only_owner]
    #[endpoint]
    fn set_penalty_percent(&self, percent: u64) {
        require!(percent < MAX_PERCENT, ERROR_PARAMETERS);
        self.penalty_percent().set(percent);
    }

    #[endpoint]
    fn set_minimum_farming_epochs(&self, epochs: Epoch) {
        self.require_caller_is_admin();
        require!(epochs <= MAX_MINIMUM_FARMING_EPOCHS, ERROR_PARAMETERS);

        self.minimum_farming_epochs().set(epochs);
    }

    #[only_owner]
    #[endpoint]
    fn set_burn_gas_limit(&self, gas_limit: u64) {
        self.burn_gas_limit().set(gas_limit);
    }

    fn should_apply_penalty(&self, entering_epoch: Epoch) -> bool {
        entering_epoch + self.minimum_farming_epochs().get() > self.blockchain().get_block_epoch()
    }

    fn get_penalty_amount(&self, amount: &BigUint) -> BigUint {
        amount * self.penalty_percent().get() / MAX_PERCENT
    }

    fn burn_penalty(
        &self,
        initial_farming_amount: &mut BigUint,
        farming_token_id: &TokenIdentifier,
        reward_token_id: &TokenIdentifier,
    ) {
        let penalty_amount = self.get_penalty_amount(initial_farming_amount);
        if penalty_amount > 0u64 {
            self.burn_farming_tokens(&penalty_amount, farming_token_id, reward_token_id);

            *initial_farming_amount -= penalty_amount;
        }
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
                .add_esdt_token_transfer(farming_token_id.clone(), 0, farming_amount.clone())
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
