elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use pair::{config::ProxyTrait as _, safe_price::ProxyTrait as _};
use pausable::ProxyTrait as _;
use simple_lock::locked_token::LockedTokenAttributes;

#[derive(TypeAbi, TopEncode, TopDecode)]
pub struct EnableSwapByUserConfig<M: ManagedTypeApi> {
    pub locked_token_id: TokenIdentifier<M>,
    pub min_locked_token_value: BigUint<M>,
    pub min_lock_period_epochs: u64,
}

#[elrond_wasm::module]
pub trait EnableSwapByUserModule: crate::factory::FactoryModule {
    #[only_owner]
    #[endpoint(configEnableByUserParameters)]
    fn config_enable_by_user_parameters(
        &self,
        locked_token_id: TokenIdentifier,
        min_locked_token_value: BigUint,
        min_lock_period_epochs: u64,
    ) {
        require!(
            locked_token_id.is_valid_esdt_identifier(),
            "Invalid locked token ID"
        );

        self.enable_swap_by_user_config()
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

        let payment = self.call_value().single_esdt();
        let config = self.try_get_config();
        require!(
            payment.token_identifier == config.locked_token_id,
            "Invalid payment token"
        );

        let own_sc_address = self.blockchain().get_sc_address();
        let locked_token_data = self.blockchain().get_esdt_token_data(
            &own_sc_address,
            &payment.token_identifier,
            payment.token_nonce,
        );
        let locked_token_attributes: LockedTokenAttributes<Self::Api> =
            locked_token_data.decode_attributes();

        let pair_lp_token_id = self.get_pair_lp_token_id(pair_address.clone());
        require!(
            locked_token_attributes.original_token_id == pair_lp_token_id,
            "Invalid locked LP token"
        );

        let lp_token_value = self.get_lp_token_value(pair_address.clone(), payment.amount);
        require!(
            lp_token_value >= config.min_locked_token_value,
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

        self.require_caller_initial_liquidity_adder(pair_address.clone());

        self.pair_resume(pair_address);
    }

    #[view(getEnableSwapByUserConfig)]
    fn try_get_config(&self) -> EnableSwapByUserConfig<Self::Api> {
        let mapper = self.enable_swap_by_user_config();
        require!(!mapper.is_empty(), "No config set");

        mapper.get()
    }

    fn get_pair_lp_token_id(&self, pair_address: ManagedAddress) -> TokenIdentifier {
        let lp_token_id: TokenIdentifier = self
            .user_pair_proxy(pair_address)
            .get_lp_token_identifier()
            .execute_on_dest_context();

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
    ) -> BigUint {
        let multi_value: MultiValue2<EsdtTokenPayment<Self::Api>, EsdtTokenPayment<Self::Api>> =
            self.user_pair_proxy(pair_address)
                .update_and_get_tokens_for_given_position_with_safe_price(lp_token_amount)
                .execute_on_dest_context();

        let (first_result, second_result) = multi_value.into_tuple();
        let whitelist = self.common_tokens_for_user_pairs();
        if whitelist.contains(&first_result.token_identifier) {
            second_result.amount
        } else if whitelist.contains(&second_result.token_identifier) {
            first_result.amount
        } else {
            sc_panic!("Invalid tokens in Pair contract");
        }
    }

    fn require_caller_initial_liquidity_adder(&self, pair_address: ManagedAddress) {
        let opt_initial_liq_adder: Option<ManagedAddress> = self
            .user_pair_proxy(pair_address)
            .get_initial_liquidity_adder()
            .execute_on_dest_context();

        match opt_initial_liq_adder {
            Some(initial_liq_adder) => {
                let caller = self.blockchain().get_caller();
                require!(
                    caller == initial_liq_adder,
                    "Caller is not the initial liq adder"
                );
            }
            None => sc_panic!("No initial liq adder was set for pair"),
        }
    }

    fn pair_resume(&self, pair_address: ManagedAddress) {
        self.user_pair_proxy(pair_address)
            .resume()
            .execute_on_dest_context_ignore_result();
    }

    #[proxy]
    fn user_pair_proxy(&self, to: ManagedAddress) -> pair::Proxy<Self::Api>;

    #[storage_mapper("enableSwapByUserConfig")]
    fn enable_swap_by_user_config(&self) -> SingleValueMapper<EnableSwapByUserConfig<Self::Api>>;

    #[view(getCommonTokensForUserPairs)]
    #[storage_mapper("commonTokensForUserPairs")]
    fn common_tokens_for_user_pairs(&self) -> UnorderedSetMapper<TokenIdentifier>;
}
