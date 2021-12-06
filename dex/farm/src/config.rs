elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::Nonce;

pub const MAX_PENALTY_PERCENT: u64 = 10_000;
pub const DEFAULT_PENALTY_PERCENT: u64 = 100;
pub const DEFAULT_MINUMUM_FARMING_EPOCHS: u8 = 3;
pub const DEFAULT_LOCKED_REWARDS_LIQUIDITY_MUTIPLIER: u8 = 2;
pub const DEFAULT_TRANSFER_EXEC_GAS_LIMIT: u64 = 35000000;
pub const DEFAULT_NFT_DEPOSIT_MAX_LEN: usize = 10;

#[derive(TopEncode, TopDecode, PartialEq, TypeAbi)]
pub enum State {
    Inactive,
    Active,
}

#[elrond_wasm::module]
pub trait ConfigModule: token_send::TokenSendModule {
    #[inline]
    fn is_active(&self) -> bool {
        let state = self.state().get();
        state == State::Active
    }

    fn require_permissions(&self) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let owner = self.owner().get();
        let router = self.router_address().get();
        require!(caller == owner || caller == router, "Permission denied");
        Ok(())
    }

    #[endpoint]
    fn set_penalty_percent(&self, percent: u64) -> SCResult<()> {
        self.require_permissions()?;
        require!(
            percent < MAX_PENALTY_PERCENT,
            "Percent cannot exceed max percent"
        );
        self.penalty_percent().set(&percent);
        Ok(())
    }

    #[endpoint]
    fn set_minimum_farming_epochs(&self, epochs: u8) -> SCResult<()> {
        self.require_permissions()?;
        self.minimum_farming_epochs().set(&epochs);
        Ok(())
    }

    #[endpoint]
    fn set_transfer_exec_gas_limit(&self, gas_limit: u64) -> SCResult<()> {
        self.require_permissions()?;
        self.transfer_exec_gas_limit().set(&gas_limit);
        Ok(())
    }

    #[endpoint]
    fn pause(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Inactive);
        Ok(())
    }

    #[endpoint]
    fn resume(&self) -> SCResult<()> {
        self.require_permissions()?;
        self.state().set(&State::Active);
        Ok(())
    }

    #[only_owner]
    #[endpoint(addAddressToWhitelist)]
    fn add_address_to_whitelist(&self, address: ManagedAddress) {
        let _ = self.whitelist().insert(address);
    }

    #[only_owner]
    #[endpoint(removeAddressFromWhitelist)]
    fn remove_address_from_whitelist(&self, address: ManagedAddress) {
        let _ = self.whitelist().swap_remove(&address);
    }

    #[inline(always)]
    fn get_farm_token_supply(&self) -> BigUint {
        self.farm_token_supply().get()
    }

    #[view(getFarmTokenSupply)]
    #[storage_mapper("farm_token_supply")]
    fn farm_token_supply(&self) -> SingleValueMapper<BigUint>;

    #[view(getLastErrorMessage)]
    #[storage_mapper("last_error_message")]
    fn last_error_message(&self) -> SingleValueMapper<ManagedBuffer>;

    #[view(getRouterManagedAddress)]
    #[storage_mapper("router_address")]
    fn router_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getState)]
    #[storage_mapper("state")]
    fn state(&self) -> SingleValueMapper<State>;

    #[view(getOwner)]
    #[storage_mapper("owner")]
    fn owner(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getFarmingTokenId)]
    #[storage_mapper("farming_token_id")]
    fn farming_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getRewardTokenId)]
    #[storage_mapper("reward_token_id")]
    fn reward_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLockedAssetFactoryManagedAddress)]
    #[storage_mapper("locked_asset_factory_address")]
    fn locked_asset_factory_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getPenaltyPercent)]
    #[storage_mapper("penalty_percent")]
    fn penalty_percent(&self) -> SingleValueMapper<u64>;

    #[view(getMinimumFarmingEpoch)]
    #[storage_mapper("minimum_farming_epochs")]
    fn minimum_farming_epochs(&self) -> SingleValueMapper<u8>;

    #[view(getPerBlockRewardAmount)]
    #[storage_mapper("per_block_reward_amount")]
    fn per_block_reward_amount(&self) -> SingleValueMapper<BigUint>;

    #[storage_mapper("produce_rewards_enabled")]
    fn produce_rewards_enabled(&self) -> SingleValueMapper<bool>;

    #[view(getLastRewardBlockNonce)]
    #[storage_mapper("last_reward_block_nonce")]
    fn last_reward_block_nonce(&self) -> SingleValueMapper<Nonce>;

    #[view(getFarmTokenId)]
    #[storage_mapper("farm_token_id")]
    fn farm_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getDivisionSafetyConstant)]
    #[storage_mapper("division_safety_constant")]
    fn division_safety_constant(&self) -> SingleValueMapper<BigUint>;

    #[view(getPairContractManagedAddress)]
    #[storage_mapper("pair_contract_address")]
    fn pair_contract_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getWhitelist)]
    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> UnorderedSetMapper<ManagedAddress>;
}
