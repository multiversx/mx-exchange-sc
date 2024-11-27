use common_structs::Epoch;

use crate::storage::DeployerType;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

const DIVISION_SAFETY_CONST: u64 = 1_000_000_000_000_000_000;

#[multiversx_sc::module]
pub trait DeployModule: crate::storage::StorageModule {
    #[endpoint(deployFarmStakingContract)]
    fn deploy_farm_staking_contract(
        &self,
        farming_token_id: TokenIdentifier,
        max_apr: BigUint,
        min_unbond_epochs: Epoch,
    ) -> ManagedAddress {
        self.require_correct_deployer_type(DeployerType::FarmStaking);
        require!(
            !self.all_used_tokens().contains(&farming_token_id),
            "Token already used"
        );

        let caller = self.get_caller_not_blacklisted();
        let deployed_sc_address = self.deploy_farm_staking_from_source(
            caller.clone(),
            farming_token_id.clone(),
            max_apr,
            min_unbond_epochs,
        );
        self.add_new_contract(&caller, &deployed_sc_address, farming_token_id);

        deployed_sc_address
    }

    fn get_caller_not_blacklisted(&self) -> ManagedAddress {
        let caller = self.blockchain().get_caller();
        let caller_id = self.address_id().get_id_or_insert(&caller);
        require!(
            !self.user_blacklist().contains(&caller_id),
            "user blacklisted"
        );

        caller
    }

    fn deploy_farm_staking_from_source(
        &self,
        caller: ManagedAddress,
        farming_token_id: TokenIdentifier,
        max_apr: BigUint,
        min_unbond_epochs: Epoch,
    ) -> ManagedAddress {
        let owner = self.blockchain().get_owner_address();

        let mut admins = MultiValueEncoded::new();
        admins.push(caller);

        let template = self.template_address().get();
        let code_metadata =
            CodeMetadata::PAYABLE_BY_SC | CodeMetadata::READABLE | CodeMetadata::UPGRADEABLE;
        let timestamp_oracle_address = self.timestamp_oracle_address().get();

        let (deployed_sc_address, ()) = self
            .farm_staking_deploy_proxy()
            .init(
                farming_token_id,
                DIVISION_SAFETY_CONST,
                max_apr,
                min_unbond_epochs,
                owner,
                timestamp_oracle_address,
                admins,
            )
            .deploy_from_source(&template, code_metadata);

        deployed_sc_address
    }

    fn add_new_contract(
        &self,
        caller: &ManagedAddress,
        deployed_sc_address: &ManagedAddress,
        farming_token_id: TokenIdentifier,
    ) {
        let contract_id = self.address_id().insert_new(deployed_sc_address);
        let _ = self.all_deployed_contracts().insert(contract_id);
        self.address_for_token(&farming_token_id).set(contract_id);
        self.token_for_address(contract_id).set(&farming_token_id);
        let _ = self.all_used_tokens().insert(farming_token_id);

        let caller_id = self.address_id().get_id_non_zero(caller);
        let _ = self.contracts_by_address(caller_id).insert(contract_id);
        self.contract_owner(contract_id).set(caller_id);
    }

    fn require_correct_deployer_type(&self, requested_type: DeployerType) {
        let deployer_type = self.deployer_type().get();
        require!(deployer_type == requested_type, "Invalid deployer type");
    }

    #[proxy]
    fn farm_staking_deploy_proxy(&self) -> farm_staking::Proxy<Self::Api>;
}
