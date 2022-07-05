elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use pausable::State;

use super::errors::*;

#[elrond_wasm::module]
pub trait ConfigModule: token_send::TokenSendModule + pausable::PausableModule {
    #[endpoint]
    fn set_extern_swap_gas_limit(&self, gas_limit: u64) {
        self.require_permissions();
        self.extern_swap_gas_limit().set(&gas_limit);
    }

    fn require_permissions(&self) {
        let caller = self.blockchain().get_caller();
        let owner = self.router_owner_address().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, ERROR_PERMISSION_DENIED);
    }

    #[endpoint(setStateActiveNoSwaps)]
    fn set_state_active_no_swaps(&self) {
        self.require_permissions();
        self.state().set(State::PartialActive);
    }

    #[endpoint(setFeePercents)]
    fn set_fee_percent(&self, total_fee_percent: u64, special_fee_percent: u64) {
        self.require_permissions();
        self.set_fee_percents(total_fee_percent, special_fee_percent);
    }

    fn set_fee_percents(&self, total_fee_percent: u64, special_fee_percent: u64) {
        require!(
            total_fee_percent >= special_fee_percent && total_fee_percent < 100_000,
            ERROR_BAD_PERCENTS
        );
        self.total_fee_percent().set(&total_fee_percent);
        self.special_fee_percent().set(&special_fee_percent);
    }

    #[view(getLpTokenIdentifier)]
    fn get_lp_token_identifier(&self) -> TokenIdentifier {
        self.lp_token_identifier().get()
    }

    #[view(getInitialLiquidtyAdder)]
    fn get_initial_liquidity_adder(&self) -> Option<ManagedAddress> {
        self.initial_liquidity_adder().get()
    }

    #[view(getTotalFeePercent)]
    #[storage_mapper("total_fee_percent")]
    fn total_fee_percent(&self) -> SingleValueMapper<u64>;

    #[view(getSpecialFee)]
    #[storage_mapper("special_fee_percent")]
    fn special_fee_percent(&self) -> SingleValueMapper<u64>;

    #[view(getRouterManagedAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getRouterOwnerManagedAddress)]
    #[storage_mapper("router_owner_address")]
    fn router_owner_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getExternSwapGasLimit)]
    #[storage_mapper("extern_swap_gas_limit")]
    fn extern_swap_gas_limit(&self) -> SingleValueMapper<u64>;

    #[storage_mapper("lpTokenIdentifier")]
    fn lp_token_identifier(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getFirstTokenId)]
    #[storage_mapper("first_token_id")]
    fn first_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getSecondTokenId)]
    #[storage_mapper("second_token_id")]
    fn second_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getTotalSupply)]
    #[storage_mapper("lp_token_supply")]
    fn lp_token_supply(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("initial_liquidity_adder")]
    fn initial_liquidity_adder(&self) -> SingleValueMapper<Option<ManagedAddress>>;

    #[view(getReserve)]
    #[storage_mapper("reserve")]
    fn pair_reserve(&self, token_id: &TokenIdentifier) -> SingleValueMapper<BigUint>;
}
