#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use energy_factory::unstake::ProxyTrait as _;
use simple_lock::locked_token::LockedTokenAttributes;

#[derive(ManagedVecItem, TopEncode, TopDecode, NestedEncode, NestedDecode, TypeAbi, Clone)]
pub struct UnstakePair<M: ManagedTypeApi> {
    pub unlock_epoch: u64,
    pub locked_tokens: EsdtTokenPayment<M>,
    pub unlocked_tokens: EsdtTokenPayment<M>,
}

#[elrond_wasm::contract]
pub trait TokenUnstakeModule: utils::UtilsModule + energy_query::EnergyQueryModule {
    /// Needs burn role for both the unlocked and locked token
    #[init]
    fn init(&self, unbond_epochs: u64, energy_factory_address: ManagedAddress) {
        self.require_sc_address(&energy_factory_address);

        self.unbond_epochs().set(unbond_epochs);
        self.energy_factory_address().set(&energy_factory_address);
    }

    #[payable("*")]
    #[endpoint(depositUserTokens)]
    fn deposit_user_tokens(&self, user: ManagedAddress) {
        let caller = self.blockchain().get_caller();
        let energy_factory_address = self.energy_factory_address().get();
        require!(
            caller == energy_factory_address,
            "Only energy factory SC can call this endpoint"
        );

        let [locked_tokens, unlocked_tokens] = self.call_value().multi_esdt();
        let current_epoch = self.blockchain().get_block_epoch();
        let unbond_epochs = self.unbond_epochs().get();
        let unlock_epoch = current_epoch + unbond_epochs;
        self.unlocked_tokens_for_user(&user)
            .update(|unstake_pairs| {
                let unstake_pair = UnstakePair {
                    unlock_epoch,
                    locked_tokens,
                    unlocked_tokens,
                };
                unstake_pairs.push(unstake_pair);
            });
    }

    #[endpoint(claimUnlockedTokens)]
    fn claim_unlocked_tokens(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let mut output_payments = ManagedVec::new();
        let mut penalty_tokens = ManagedVec::new();
        self.unlocked_tokens_for_user(&caller)
            .update(|user_entries| {
                while !user_entries.is_empty() {
                    let entry = user_entries.get(0);
                    if current_epoch < entry.unlock_epoch {
                        break;
                    }

                    let locked_tokens = entry.locked_tokens;
                    let unlocked_tokens = entry.unlocked_tokens;

                    // we only burn the tokens that are not unlocked
                    // the rest are sent back as penalty
                    let locked_tokens_burn_amount = unlocked_tokens.amount.clone();
                    self.send().esdt_local_burn(
                        &locked_tokens.token_identifier,
                        locked_tokens.token_nonce,
                        &locked_tokens_burn_amount,
                    );

                    let penalty_amount = &locked_tokens.amount - &unlocked_tokens.amount;
                    if penalty_amount > 0 {
                        let penalty = EsdtTokenPayment::new(
                            locked_tokens.token_identifier,
                            locked_tokens.token_nonce,
                            penalty_amount,
                        );
                        penalty_tokens.push(penalty);
                    }

                    output_payments.push(unlocked_tokens);
                    user_entries.remove(0);
                }
            });

        if !output_payments.is_empty() {
            self.send().direct_multi(&caller, &output_payments);
        }

        if !penalty_tokens.is_empty() {
            let sc_address = self.energy_factory_address().get();
            let _: IgnoreValue = self
                .energy_factory_proxy(sc_address)
                .finalize_unstake()
                .with_multi_token_transfer(penalty_tokens)
                .execute_on_dest_context();
        }

        output_payments.into()
    }

    #[endpoint(cancelUnbond)]
    fn cancel_unbond(&self) -> MultiValueEncoded<EsdtTokenPayment> {
        let caller = self.blockchain().get_caller();
        let current_epoch = self.blockchain().get_block_epoch();
        let own_sc_address = self.blockchain().get_sc_address();

        let mut output_payments = ManagedVec::new();
        let mut energy = self.get_energy_entry(&caller);

        let entries_mapper = self.unlocked_tokens_for_user(&caller);
        let user_entries = entries_mapper.get();
        require!(!user_entries.is_empty(), "No tokens to unbond");

        for entry in &user_entries {
            let locked_tokens = entry.locked_tokens;
            let token_data = self.blockchain().get_esdt_token_data(
                &own_sc_address,
                &locked_tokens.token_identifier,
                locked_tokens.token_nonce,
            );
            let attributes: LockedTokenAttributes<Self::Api> = token_data.decode_attributes();
            if attributes.unlock_epoch >= current_epoch {
                energy.add_after_token_lock(
                    &locked_tokens.amount,
                    attributes.unlock_epoch,
                    current_epoch,
                );
            } else {
                // account for energy refund on unlock
                let epoch_diff = current_epoch - attributes.unlock_epoch;
                let energy_to_reduce = &locked_tokens.amount * epoch_diff;
                energy.add_energy_raw(locked_tokens.amount.clone(), BigUint::zero());
                energy.remove_energy_raw(BigUint::zero(), energy_to_reduce);
            }

            output_payments.push(locked_tokens);

            self.send().esdt_local_burn(
                &entry.unlocked_tokens.token_identifier,
                0,
                &entry.unlocked_tokens.amount,
            );
        }

        entries_mapper.clear();

        self.send().direct_multi(&caller, &output_payments);

        let sc_address = self.energy_factory_address().get();
        let _: IgnoreValue = self
            .energy_factory_proxy(sc_address)
            .revert_unstake(caller, energy)
            .execute_on_dest_context();

        output_payments.into()
    }

    #[view(getUnbondEpochs)]
    #[storage_mapper("unbondEpochs")]
    fn unbond_epochs(&self) -> SingleValueMapper<u64>;

    #[view(getUnlockedTokensForUser)]
    #[storage_mapper("unlockedTokensForUser")]
    fn unlocked_tokens_for_user(
        &self,
        address: &ManagedAddress,
    ) -> SingleValueMapper<ManagedVec<UnstakePair<Self::Api>>>;
}
