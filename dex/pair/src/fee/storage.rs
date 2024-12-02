use common_structs::{Percent, TokenPair};

multiversx_sc::imports!();

#[multiversx_sc::module]
pub trait StorageModule {
    #[view(getFeesCollectorAddress)]
    #[storage_mapper("feesCollectorAddress")]
    fn fees_collector_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getFeesCollectorCutPercentage)]
    #[storage_mapper("feesCollectorCutPercentage")]
    fn fees_collector_cut_percentage(&self) -> SingleValueMapper<Percent>;

    #[storage_mapper("fee_destination")]
    fn destination_map(&self) -> MapMapper<ManagedAddress, TokenIdentifier>;

    #[storage_mapper("trusted_swap_pair")]
    fn trusted_swap_pair(&self) -> MapMapper<TokenPair<Self::Api>, ManagedAddress>;

    #[storage_mapper("whitelist")]
    fn whitelist(&self) -> SetMapper<ManagedAddress>;
}
