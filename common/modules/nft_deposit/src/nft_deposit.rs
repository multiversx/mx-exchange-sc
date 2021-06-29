#![no_std]

elrond_wasm::imports!();
elrond_wasm::derive_imports!();

use common_structs::{GenericEsdtAmountPair, Nonce};

#[elrond_wasm_derive::module]
pub trait NftDepositModule: token_send::TokenSendModule {
    #[payable("*")]
    #[endpoint(depositToken)]
    fn deposit_token(
        &self,
        #[payment_token] payment_token_id: TokenIdentifier,
        #[payment_nonce] payment_token_nonce: Nonce,
        #[payment_amount] payment_amount: Self::BigUint,
    ) -> SCResult<()> {
        require!(payment_amount != 0, "Cannot deposit 0 tokens");
        require!(payment_token_nonce != 0, "Cannot deposit fungible tokens");

        let payment = GenericEsdtAmountPair {
            token_id: payment_token_id,
            token_nonce: payment_token_nonce,
            amount: payment_amount,
        };

        let mut index = 1;
        let mut entry_updated = false;
        let caller = self.blockchain().get_caller();
        let deposit_len = self.nft_deposit(&caller).len();

        while index <= deposit_len {
            let mut entry = self.nft_deposit(&caller).get(index);

            if self.equal_token_type(&entry, &payment) {
                entry.amount = &entry.amount + &payment.amount;
                self.nft_deposit(&caller).set(index, &entry);
                entry_updated = true;
                break;
            }

            index = index + 1;
        }

        if !entry_updated {
            require!(deposit_len + 1 < self.nft_deposit_max_len().get(), "Deposit is full");
            self.nft_deposit(&caller).push(&payment);
        }

        Ok(())
    }

    #[endpoint(withdrawAllTokensFromDeposit)]
    fn withdraw_all_tokens_from_deposit(
        &self,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        let caller = self.blockchain().get_caller();
        let mut deposit_len = self.nft_deposit(&caller).len();

        while deposit_len > 0 {
            self.withdraw_token(
                deposit_len,
                &caller,
                &opt_accept_funds_func,
            )?;
            deposit_len = deposit_len - 1;
        }

        Ok(())
    }

    #[endpoint(withdrawTokenFromDeposit)]
    fn withdraw_token_from_deposit(
        &self,
        index: usize,
        #[var_args] opt_accept_funds_func: OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        self.withdraw_token(
            index,
            &self.blockchain().get_caller(),
            &opt_accept_funds_func,
        )
    }

    fn withdraw_token(
        &self,
        index: usize,
        caller: &Address,
        opt_accept_funds_func: &OptionalArg<BoxedBytes>,
    ) -> SCResult<()> {
        require!(
            index >= 1 && index <= self.nft_deposit(caller).len(),
            "Out of range index"
        );

        let entry = self.nft_deposit(caller).get(index);
        self.nft_deposit(caller).clear_entry(index);
        self.send_nft_tokens(
            &entry.token_id,
            entry.token_nonce,
            &entry.amount,
            caller,
            opt_accept_funds_func,
        );

        Ok(())
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
    ) -> VecMapper<Self::Storage, GenericEsdtAmountPair<Self::BigUint>>;

    #[view(getnftDepositMaxLen)]
    #[storage_mapper("nft_deposit_max_len")]
    fn nft_deposit_max_len(&self) -> SingleValueMapper<Self::Storage, usize>;
}
