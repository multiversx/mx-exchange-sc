#![no_std]

multiversx_sc::imports!();

pub mod basic_lock_unlock;
pub mod error_messages;
pub mod locked_token;
pub mod proxy_farm;
pub mod proxy_lp;

#[multiversx_sc::contract]
pub trait SimpleLockLegacy:
    basic_lock_unlock::BasicLockUnlock
    + locked_token::LockedTokenModule
    + proxy_lp::ProxyLpModule
    + proxy_farm::ProxyFarmModule
{
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(unlockTokens)]
    fn unlock_tokens_endpoint(
        &self,
        opt_destination: OptionalValue<ManagedAddress>,
    ) -> EgldOrEsdtTokenPayment<Self::Api> {
        let payment = self.call_value().single_esdt();
        let dest_address = self.dest_from_optional(opt_destination);
        self.unlock_and_send(&dest_address, payment)
    }

    fn dest_from_optional(&self, opt_destination: OptionalValue<ManagedAddress>) -> ManagedAddress {
        match opt_destination {
            OptionalValue::Some(dest) => dest,
            OptionalValue::None => self.blockchain().get_caller(),
        }
    }
}
