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

#[multiversx_sc::contract]
pub trait PriceDiscoveryV1 {
    #[init]
    fn init(&self) {}

    #[payable("*")]
    #[endpoint]
    fn deposit(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("*")]
    #[endpoint]
    fn withdraw(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[payable("*")]
    #[endpoint]
    fn redeem(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getCurrentPrice)]
    fn calculate_price(&self) -> BigUint {
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[view(getCurrentPhase)]
    fn get_current_phase(&self) -> Phase<Self::Api> {
        sc_panic!("This is a legacy contract, should not be interacted with");
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
        sc_panic!("This is a legacy contract, should not be interacted with");
    }

    #[only_owner]
    #[endpoint(createInitialRedeemTokens)]
    fn create_initial_redeem_tokens(&self) {
        sc_panic!("This is a legacy contract, should not be interacted with");
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
}
