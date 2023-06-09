multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use pair::config::ProxyTrait as _;
use pausable::{ProxyTrait as _, State};
use simple_lock::locked_token::LockedTokenAttributes;

use crate::{DEFAULT_SPECIAL_FEE_PERCENT, USER_DEFINED_TOTAL_FEE_PERCENT};

static PAIR_LP_TOKEN_ID_STORAGE_KEY: &[u8] = b"lpTokenIdentifier";
static PAIR_INITIAL_LIQ_ADDER_STORAGE_KEY: &[u8] = b"initial_liquidity_adder";
static PAIR_STATE_STORAGE_KEY: &[u8] = b"state";

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct EnableSwapByUserConfig<M: ManagedTypeApi> {
    pub locked_token_id: TokenIdentifier<M>,
    pub min_locked_token_value: BigUint<M>,
    pub min_lock_period_epochs: u64,
}

pub struct SafePriceResult<M: ManagedTypeApi> {
    pub first_token_id: TokenIdentifier<M>,
    pub second_token_id: TokenIdentifier<M>,
    pub common_token_id: TokenIdentifier<M>,
    pub safe_price_in_common_token: BigUint<M>,
}

#[multiversx_sc::module]
pub trait EnableSwapByUserModule:
    crate::factory::FactoryModule + crate::events::EventsModule
{
    #[only_owner]
    #[endpoint(configEnableByUserParameters)]
    fn config_enable_by_user_parameters(
        &self,
        common_token_id: TokenIdentifier,
        locked_token_id: TokenIdentifier,
        min_locked_token_value: BigUint,
        min_lock_period_epochs: u64,
    ) {
        require!(
            common_token_id.is_valid_esdt_identifier(),
            "Invalid locked token ID"
        );
        require!(
            locked_token_id.is_valid_esdt_identifier(),
            "Invalid locked token ID"
        );

        let whitelist = self.common_tokens_for_user_pairs();
        require!(
            whitelist.contains(&common_token_id),
            "Common token not whitelisted"
        );

        self.enable_swap_by_user_config(&common_token_id)
            .set(&EnableSwapByUserConfig {
                locked_token_id,
                min_locked_token_value,
                min_lock_period_epochs,
            });
    }

    #[only_owner]
    #[endpoint(addCommonTokensForUserPairs)]
    fn add_common_tokens_for_user_pairs(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut whitelist = self.common_tokens_for_user_pairs();
        for token in tokens {
            require!(token.is_valid_esdt_identifier(), "Invalid token ID");
            let _ = whitelist.insert(token);
        }
    }

    #[only_owner]
    #[endpoint(removeCommonTokensForUserPairs)]
    fn remove_common_tokens_for_user_pairs(&self, tokens: MultiValueEncoded<TokenIdentifier>) {
        let mut whitelist = self.common_tokens_for_user_pairs();
        for token in tokens {
            let _ = whitelist.swap_remove(&token);
        }
    }

    #[payable("*")]
    #[endpoint(setSwapEnabledByUser)]
    fn set_swap_enabled_by_user(&self, pair_address: ManagedAddress) {
        self.check_is_pair_sc(&pair_address);
        self.require_state_active_no_swaps(&pair_address);

        let payment = self.call_value().single_esdt();

        let own_sc_address = self.blockchain().get_sc_address();
        let locked_token_data = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &payment.token_identifier,
            payment.token_nonce,
        );
        let locked_token_attributes: LockedTokenAttributes<Self::Api> =
            locked_token_data.decode_attributes();

        let pair_lp_token_id = self.get_pair_lp_token_id(&pair_address);
        require!(
            locked_token_attributes.original_token_id == pair_lp_token_id,
            "Invalid locked LP token"
        );

        let locked_lp_token_amount = payment.amount.clone();
        let lp_token_safe_price_result =
            self.get_lp_token_value(pair_address.clone(), locked_lp_token_amount);
        let config = self.try_get_config(&lp_token_safe_price_result.common_token_id);
        require!(
            payment.token_identifier == config.locked_token_id,
            "Invalid locked token"
        );
        require!(
            lp_token_safe_price_result.safe_price_in_common_token >= config.min_locked_token_value,
            "Not enough value locked"
        );

        let current_epoch = self.blockchain().get_block_epoch();
        let locked_epochs = if current_epoch < locked_token_attributes.unlock_epoch {
            locked_token_attributes.unlock_epoch - current_epoch
        } else {
            0
        };
        require!(
            locked_epochs >= config.min_lock_period_epochs,
            "Token not locked for long enough"
        );

        let caller = self.blockchain().get_caller();
        self.require_caller_initial_liquidity_adder(&pair_address, &caller);

        self.set_fee_percents(pair_address.clone());
        self.pair_resume(pair_address.clone());

        self.send().direct_esdt(
            &caller,
            &payment.token_identifier,
            payment.token_nonce,
            &payment.amount,
        );

        self.emit_user_swaps_enabled_event(
            caller,
            lp_token_safe_price_result.first_token_id,
            lp_token_safe_price_result.second_token_id,
            pair_address,
        );
    }

    #[view(getEnableSwapByUserConfig)]
    fn try_get_config(&self, token_id: &TokenIdentifier) -> EnableSwapByUserConfig<Self::Api> {
        let mapper = self.enable_swap_by_user_config(token_id);
        require!(!mapper.is_empty(), "No config set");

        mapper.get()
    }

    fn get_pair_lp_token_id(&self, pair_address: &ManagedAddress) -> TokenIdentifier {
        let lp_token_id: TokenIdentifier =
            self.read_storage_from_pair(pair_address, PAIR_LP_TOKEN_ID_STORAGE_KEY);
        require!(
            lp_token_id.is_valid_esdt_identifier(),
            "Invalid LP token received from pair"
        );

        lp_token_id
    }

    fn get_lp_token_value(
        &self,
        pair_address: ManagedAddress,
        lp_token_amount: BigUint,
    ) -> SafePriceResult<Self::Api> {
        let multi_value: MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> =
            self.user_pair_proxy(pair_address)
                .get_tokens_for_given_position(lp_token_amount)
                .execute_on_dest_context();

        let (first_result, second_result) = multi_value.into_tuple();
        let mut safe_price_result = SafePriceResult {
            first_token_id: first_result.token_identifier.clone(),
            second_token_id: second_result.token_identifier.clone(),
            common_token_id: first_result.token_identifier,
            safe_price_in_common_token: BigUint::zero(),
        };
        let whitelist = self.common_tokens_for_user_pairs();
        if whitelist.contains(&safe_price_result.first_token_id) {
            safe_price_result.safe_price_in_common_token = first_result.amount;
        } else if whitelist.contains(&second_result.token_identifier) {
            safe_price_result.common_token_id = second_result.token_identifier;
            safe_price_result.safe_price_in_common_token = second_result.amount;
        } else {
            sc_panic!("Invalid tokens in Pair contract");
        };

        safe_price_result
    }

    fn require_state_active_no_swaps(&self, pair_address: &ManagedAddress) {
        let state: State = self.read_storage_from_pair(pair_address, PAIR_STATE_STORAGE_KEY);
        require!(
            state == State::PartialActive,
            "Pair not in ActiveNoSwaps state"
        );
    }

    fn require_caller_initial_liquidity_adder(
        &self,
        pair_address: &ManagedAddress,
        caller: &ManagedAddress,
    ) {
        let opt_initial_liq_adder: Option<ManagedAddress> =
            self.read_storage_from_pair(pair_address, PAIR_INITIAL_LIQ_ADDER_STORAGE_KEY);

        match opt_initial_liq_adder {
            Some(initial_liq_adder) => {
                require!(
                    caller == &initial_liq_adder,
                    "Caller is not the initial liq adder"
                );
            }
            None => sc_panic!("No initial liq adder was set for pair"),
        }
    }

    fn set_fee_percents(&self, pair_address: ManagedAddress) {
        let _: IgnoreValue = self
            .user_pair_proxy(pair_address)
            .set_fee_percent(USER_DEFINED_TOTAL_FEE_PERCENT, DEFAULT_SPECIAL_FEE_PERCENT)
            .execute_on_dest_context();
    }

    fn pair_resume(&self, pair_address: ManagedAddress) {
        let _: IgnoreValue = self
            .user_pair_proxy(pair_address)
            .resume()
            .execute_on_dest_context();
    }

    fn read_storage_from_pair<T: TopDecode>(
        &self,
        pair_address: &ManagedAddress,
        storage_key: &[u8],
    ) -> T {
        let key_buffer = ManagedBuffer::new_from_bytes(storage_key);
        self.storage_raw()
            .read_from_address(pair_address, key_buffer)
    }

    #[proxy]
    fn user_pair_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[storage_mapper("enableSwapByUserConfig")]
    fn enable_swap_by_user_config(
        &self,
        token_id: &TokenIdentifier,
    ) -> SingleValueMapper<EnableSwapByUserConfig<Self::Api>>;

    #[view(getCommonTokensForUserPairs)]
    #[storage_mapper("commonTokensForUserPairs")]
    fn common_tokens_for_user_pairs(&self) -> UnorderedSetMapper<TokenIdentifier>;
}
