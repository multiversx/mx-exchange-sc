#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{GenericEsdtAmountPair, Nonce};

#[elrond_wasm_derive::module]
pub trait NftDepositModule: token_send::TokenSendModule + token_supply::TokenSupplyModule {
    fn deposit_tokens(
        &self,
        payment_token_id: TokenIdentifier,
        payment_token_nonce: Nonce,
        payment_amount: Self::BigUint,
    ) -> SCResult<()> {
        require!(payment_amount != 0, "Cannot deposit 0 tokens");
        require!(payment_token_nonce != 0, "Cannot deposit fungible tokens");
        require!(
            self.nft_deposit_accepted_token_ids()
                .contains(&payment_token_id),
            "Not an accepted token id"
        );

        let payment = GenericEsdtAmountPair {
            token_id: payment_token_id,
            token_nonce: payment_token_nonce,
            amount: payment_amount,
        };

        let mut entry_updated = false;
        let caller = self.blockchain().get_caller();
        let mut deposit = self.nft_deposit(&caller).get();

        for entry in deposit.iter_mut() {
            if self.equal_token_type(entry, &payment) {
                entry.amount += &payment.amount;
                entry_updated = true;
                break;
            }
        }

        if !entry_updated {
            require!(
                deposit.len() < self.nft_deposit_max_len().get(),
                "Deposit is full"
            );
            deposit.push(payment);
        }

        self.nft_deposit(&caller).set(&deposit);
        Ok(())
    }

    #[endpoint(withdrawAllTokensFromDeposit)]
    fn withdraw_all_tokens_from_deposit(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let deposit = self.nft_deposit(&caller).get();

        self.nft_deposit(&caller).clear();
        deposit.iter().for_each(|entry| {
            self.send_nft_tokens(
                &entry.token_id,
                entry.token_nonce,
                &entry.amount,
                &caller,
                &opt_accept_funds_func,
            )
        });

        Ok(())
    }

    #[endpoint(withdrawTokenFromDeposit)]
    fn withdraw_token_from_deposit(
        &self,
        index: usize,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let mut deposit = self.nft_deposit(&caller).get();
        require!(index > 0 && index < deposit.len(), "Index out of range");

        let entry = deposit.remove(index);
        self.nft_deposit(&caller).set(&deposit);

        self.send_nft_tokens(
            &entry.token_id,
            entry.token_nonce,
            &entry.amount,
            &caller,
            &opt_accept_funds_func,
        );

        Ok(())
    }

    fn burn_deposit_tokens(
        &self,
        caller: &Address,
        deposit: &[GenericEsdtAmountPair<Self::BigUint>],
    ) {
        deposit.iter().for_each(|entry| {
            self.nft_burn_tokens(&entry.token_id, entry.token_nonce, &entry.amount)
        });
        self.nft_deposit(caller).clear();
    }

    fn equal_token_type(
        &self,
        first: &GenericEsdtAmountPair<Self::BigUint>,
        second: &GenericEsdtAmountPair<Self::BigUint>,
    ) -> bool {
        first.token_id == second.token_id && first.token_nonce == second.token_nonce
    }

    #[view(getnftDeposit)]
    #[storage_mapper("nft_deposit")]
    fn nft_deposit(
        &self,
        address: &Address,
    ) -> SingleValueMapper<Self::Storage, Vec<GenericEsdtAmountPair<Self::BigUint>>>;

    #[view(getnftDepositMaxLen)]
    #[storage_mapper("nft_deposit_max_len")]
    fn nft_deposit_max_len(&self) -> SingleValueMapper<Self::Storage, usize>;

    #[view(getNftDepositAcceptedTokenIds)]
    #[storage_mapper("nft_deposit_accepted_token_ids")]
    fn nft_deposit_accepted_token_ids(&self) -> SetMapper<Self::Storage, TokenIdentifier>;
}
