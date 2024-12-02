multiversx_sc::imports!();
multiversx_sc::derive_imports!();

use common_structs::Week;
use energy_query::Energy;
use multiversx_sc_scenario::{managed_biguint, DebugApi};
use weekly_rewards_splitting::ClaimProgress;

#[derive(TypeAbi, TopEncode, Clone, PartialEq, Debug)]
pub struct OldClaimProgress<M: ManagedTypeApi> {
    pub energy: Energy<M>,
    pub week: Week,
}

#[test]
fn decode_old_claim_progress_to_new_test() {
    DebugApi::dummy();

    let old_progress = OldClaimProgress {
        energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
        week: 2,
    };
    let mut old_progress_encoded = ManagedBuffer::<DebugApi>::new();
    let _ = old_progress.top_encode(&mut old_progress_encoded);

    let new_progress_decoded = ClaimProgress::top_decode(old_progress_encoded).unwrap();
    assert_eq!(
        new_progress_decoded,
        ClaimProgress {
            energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
            week: 2,
            enter_timestamp: 0,
        }
    );
}

#[test]
fn encoded_decode_new_progress_test() {
    DebugApi::dummy();

    let new_progress = ClaimProgress {
        energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
        week: 2,
        enter_timestamp: 0,
    };
    let mut new_progress_encoded = ManagedBuffer::<DebugApi>::new();
    let _ = new_progress.top_encode(&mut new_progress_encoded);
    let new_progress_decoded = ClaimProgress::top_decode(new_progress_encoded).unwrap();
    assert_eq!(
        new_progress_decoded,
        ClaimProgress {
            energy: Energy::new(BigInt::<DebugApi>::zero(), 10, managed_biguint!(20)),
            week: 2,
            enter_timestamp: 0,
        }
    );
}
