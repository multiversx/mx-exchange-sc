use common_structs::Percent;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

#[derive(TopEncode, TopDecode, NestedEncode, NestedDecode, PartialEq, TypeAbi)]
pub struct PairTokens<M: ManagedTypeApi> {
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
}

pub struct CreatePairArgs<'a, M: ManagedTypeApi> {
    pub first_token_id: &'a TokenIdentifier<M>,
    pub second_token_id: &'a TokenIdentifier<M>,
    pub owner: &'a ManagedAddress<M>,
    pub total_fee_percent: Percent,
    pub special_fee_percent: Percent,
    pub initial_liquidity_adder: &'a ManagedAddress<M>,
    pub admins: MultiValueEncoded<M, ManagedAddress<M>>,
}

pub const DEFAULT_TOTAL_FEE_PERCENT: Percent = 300;
pub const DEFAULT_SPECIAL_FEE_PERCENT: Percent = 50;
pub const MAX_TOTAL_FEE_PERCENT: Percent = 100_000;
pub const USER_DEFINED_TOTAL_FEE_PERCENT: Percent = 1_000;

pub type FeePercentArgType = OptionalValue<MultiValue2<Percent, Percent>>;

pub struct FeePercentResult {
    pub total_fee_percent_requested: Percent,
    pub special_fee_percent_requested: Percent,
}

#[multiversx_sc::module]
pub trait CreateModule:
    crate::config::ConfigModule
    + pair::read_pair_storage::ReadPairStorageModule
    + crate::temp_owner::TempOwnerModule
    + crate::events::EventsModule
    + crate::state::StateModule
    + crate::views::ViewsModule
{
    #[allow_multiple_var_args]
    #[endpoint(createPair)]
    fn create_pair_endpoint(
        &self,
        first_token_id: TokenIdentifier,
        second_token_id: TokenIdentifier,
        initial_liquidity_adder: ManagedAddress,
        opt_fee_percents: FeePercentArgType,
        mut admins: MultiValueEncoded<ManagedAddress>,
    ) -> ManagedAddress {
        self.require_active();

        let owner = self.owner().get();
        let caller = self.blockchain().get_caller();
        if caller != owner {
            self.require_pair_creation_enabled();
        }

        require!(first_token_id != second_token_id, "Identical tokens");
        require!(
            first_token_id.is_valid_esdt_identifier(),
            "First Token ID is not a valid esdt token ID"
        );
        require!(
            second_token_id.is_valid_esdt_identifier(),
            "Second Token ID is not a valid esdt token ID"
        );

        let pair_address = self.get_pair(first_token_id.clone(), second_token_id.clone());
        require!(pair_address.is_zero(), "Pair already exists");

        let (total_fee_percent_requested, special_fee_percent_requested) = if caller == owner {
            let fee_percents = self.get_owner_set_fee_percents(opt_fee_percents);

            (
                fee_percents.total_fee_percent_requested,
                fee_percents.special_fee_percent_requested,
            )
        } else {
            (DEFAULT_TOTAL_FEE_PERCENT, DEFAULT_SPECIAL_FEE_PERCENT)
        };

        admins.push(caller.clone());

        let address = self.create_pair(CreatePairArgs {
            first_token_id: &first_token_id,
            second_token_id: &second_token_id,
            owner: &owner,
            total_fee_percent: total_fee_percent_requested,
            special_fee_percent: special_fee_percent_requested,
            initial_liquidity_adder: &initial_liquidity_adder,
            admins,
        });

        self.emit_create_pair_event(
            &caller,
            &first_token_id,
            &second_token_id,
            total_fee_percent_requested,
            special_fee_percent_requested,
            &address,
        );

        address
    }

    fn get_owner_set_fee_percents(&self, opt_fee_percents: FeePercentArgType) -> FeePercentResult {
        match opt_fee_percents {
            OptionalValue::Some(fee_percents_multi_arg) => {
                let fee_percents_tuple = fee_percents_multi_arg.into_tuple();
                let total_fee_percent_requested = fee_percents_tuple.0;
                let special_fee_percent_requested = fee_percents_tuple.1;
                require!(
                    total_fee_percent_requested >= special_fee_percent_requested
                        && total_fee_percent_requested < MAX_TOTAL_FEE_PERCENT,
                    "Bad percents"
                );

                FeePercentResult {
                    total_fee_percent_requested,
                    special_fee_percent_requested,
                }
            }
            OptionalValue::None => sc_panic!("Bad percents length"),
        }
    }

    fn create_pair(&self, args: CreatePairArgs<Self::Api>) -> ManagedAddress {
        require!(
            !self.pair_template_address().is_empty(),
            "pair contract template is empty"
        );

        let template_addr = self.pair_template_address().get();
        let code_metadata = self.get_default_code_metadata();
        let (new_address, ()) = self
            .pair_contract_deploy_proxy()
            .init(
                args.first_token_id,
                args.second_token_id,
                self.blockchain().get_sc_address(),
                args.owner,
                args.total_fee_percent,
                args.special_fee_percent,
                args.initial_liquidity_adder,
                args.admins,
            )
            .deploy_from_source(&template_addr, code_metadata);

        self.pair_map().insert(
            PairTokens {
                first_token_id: args.first_token_id.clone(),
                second_token_id: args.second_token_id.clone(),
            },
            new_address.clone(),
        );
        self.pair_temporary_owner().insert(
            new_address.clone(),
            (
                self.blockchain().get_caller(),
                self.blockchain().get_block_nonce(),
            ),
        );

        new_address
    }

    fn get_default_code_metadata(&self) -> CodeMetadata {
        CodeMetadata::UPGRADEABLE | CodeMetadata::READABLE | CodeMetadata::PAYABLE_BY_SC
    }

    #[proxy]
    fn pair_contract_deploy_proxy(&self) -> pair::Proxy<Self::Api>;
}
