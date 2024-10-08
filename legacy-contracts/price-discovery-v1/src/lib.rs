#![no_std]

use multiversx_sc::derive_imports::*;
use multiversx_sc::imports::*;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, PartialEq)]
pub enum Phase<M: ManagedTypeApi> {
    Idle,
    NoPenalty,
    LinearIncreasingPenalty { penalty_percentage: BigUint<M> },
    OnlyWithdrawFixedPenalty { penalty_percentage: BigUint<M> },
    Redeem,
}

static ERROR_LEGACY_CONTRACT: &[u8] = b"This is a no-code version of a legacy contract. The logic of the endpoints has not been implemented.";

#[multiversx_sc::contract]
pub trait PriceDiscoveryV1 {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint]
    fn withdraw(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[payable("*")]
    #[endpoint]
    fn redeem(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getCurrentPrice)]
    fn calculate_price(&self) -> BigUint {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getCurrentPhase)]
    fn get_current_phase(&self) -> Phase<Self::Api> {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[payable("EGLD")]
    #[endpoint(issueRedeemToken)]
    fn issue_redeem_token(
        &self,
        _token_name: ManagedBuffer,
        _token_ticker: ManagedBuffer,
        _nr_decimals: usize,
    ) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(createInitialRedeemTokens)]
    fn create_initial_redeem_tokens(&self) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setLockingScAddress)]
    fn set_locking_sc_address(&self, _new_address: ManagedAddress) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[only_owner]
    #[endpoint(setUnlockEpoch)]
    fn set_unlock_epoch(&self, _new_epoch: u64) {
        sc_panic!(ERROR_LEGACY_CONTRACT);
    }

    #[view(getLaunchedTokenId)]
    #[storage_mapper("launchedTokenId")]
    fn launched_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getAcceptedTokenId)]
    #[storage_mapper("acceptedTokenId")]
    fn accepted_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getLaunchedTokenBalance)]
    #[storage_mapper("launchedTokenBalance")]
    fn launched_token_balance(&self) -> SingleValueMapper<BigUint>;

    #[view(getAcceptedTokenBalance)]
    #[storage_mapper("acceptedTokenBalance")]
    fn accepted_token_balance(&self) -> SingleValueMapper<BigUint>;

    #[view(getStartBlock)]
    #[storage_mapper("startBlock")]
    fn start_block(&self) -> SingleValueMapper<u64>;

    #[view(getEndBlock)]
    #[storage_mapper("endBlock")]
    fn end_block(&self) -> SingleValueMapper<u64>;

    #[view(getRedeemTokenId)]
    #[storage_mapper("redeemTokenId")]
    fn redeem_token_id(&self) -> SingleValueMapper<TokenIdentifier>;

    #[view(getRedeemTokenTotalCirculatingSupply)]
    #[storage_mapper("totalCirculatingSupply")]
    fn redeem_token_total_circulating_supply(&self, token_nonce: u64)
        -> SingleValueMapper<BigUint>;

    #[view(getMinLaunchedTokenPrice)]
    #[storage_mapper("minLaunchedTokenPrice")]
    fn min_launched_token_price(&self) -> SingleValueMapper<BigUint>;

    #[view(getPricePrecision)]
    #[storage_mapper("pricePrecision")]
    fn price_precision(&self) -> SingleValueMapper<u64>;

    #[view(getNoLimitPhaseDurationBlocks)]
    #[storage_mapper("noLimitPhaseDurationBlocks")]
    fn no_limit_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getLinearPenaltyPhaseDurationBlocks)]
    #[storage_mapper("linearPenaltyPhaseDurationBlocks")]
    fn linear_penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getFixedPenaltyPhaseDurationBlocks)]
    #[storage_mapper("fixedPenaltyPhaseDurationBlocks")]
    fn fixed_penalty_phase_duration_blocks(&self) -> SingleValueMapper<u64>;

    #[view(getPenaltyMinPercentage)]
    #[storage_mapper("penaltyMinPercentage")]
    fn penalty_min_percentage(&self) -> SingleValueMapper<BigUint>;

    #[view(getPenaltyMaxPercentage)]
    #[storage_mapper("penaltyMaxPercentage")]
    fn penalty_max_percentage(&self) -> SingleValueMapper<BigUint>;

    #[view(getFixedPenaltyPercentage)]
    #[storage_mapper("fixedPenaltyPercentage")]
    fn fixed_penalty_percentage(&self) -> SingleValueMapper<BigUint>;

    #[view(getLockingScAddress)]
    #[storage_mapper("lockingScAddress")]
    fn locking_sc_address(&self) -> SingleValueMapper<ManagedAddress>;

    #[view(getUnlockEpoch)]
    #[storage_mapper("unlockEpoch")]
    fn unlock_epoch(&self) -> SingleValueMapper<u64>;
}
