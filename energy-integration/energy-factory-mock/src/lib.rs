#![no_std]

multiversx_sc::imports!();

use energy_query::Energy;

#[multiversx_sc::contract]
pub trait EnergyFactoryMock {
    #[init]
    fn init(&self) {}

    #[endpoint(setUserEnergy)]
    fn set_user_energy(
        &self,
        user: ManagedAddress,
        energy_amount: BigUint,
        total_locked_tokens: BigUint,
    ) {
        let current_epoch = self.blockchain().get_block_epoch();
        self.user_energy(&user).set(&Energy::new(
            BigInt::from(energy_amount),
            current_epoch,
            total_locked_tokens,
        ));
    }

    #[view(getEnergyAmountForUser)]
    fn get_energy_amount_for_user(&self, user: ManagedAddress) -> BigUint {
        self.get_energy_entry_for_user(user).get_energy_amount()
    }

    #[view(getEnergyEntryForUser)]
    fn get_energy_entry_for_user(&self, user: ManagedAddress) -> Energy<Self::Api> {
        let current_epoch = self.blockchain().get_block_epoch();
        let mapper = self.user_energy(&user);
        if !mapper.is_empty() {
            let mut energy = mapper.get();
            energy.deplete(current_epoch);

            energy
        } else {
            Energy::new_zero_energy(current_epoch)
        }
    }

    #[endpoint(setUserEnergyAfterLockedTokenTransfer)]
    fn set_user_energy_after_locked_token_transfer(
        &self,
        user: ManagedAddress,
        energy: Energy<Self::Api>,
    ) {
        self.user_energy(&user).set(&energy);
    }

    #[storage_mapper("userEnergy")]
    fn user_energy(&self, user: &ManagedAddress) -> SingleValueMapper<Energy<Self::Api>>;

    #[storage_mapper("lockedTokenId")]
    fn locked_token(&self) -> NonFungibleTokenMapper;
}
