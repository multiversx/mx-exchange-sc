mod farm_setup;

use std::ops::Mul;

use farm_setup::farm_rewards_distr_setup::*;
use multiversx_sc_scenario::rust_biguint;

#[test]
fn test_overview() {
    let per_block_reward_amount = rust_biguint!(100);
    let mut farm_setup = FarmRewardsDistrSetup::new(farm::contract_obj, per_block_reward_amount);
    let alice = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    let bob = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    let eve = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    farm_setup.step(
        3,
        Action::EnterFarm(alice.clone(), rust_biguint!(1_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(1_000)),
    );
    farm_setup.step(
        3,
        Action::EnterFarm(bob.clone(), rust_biguint!(2_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(3_000)),
    );
    farm_setup.step(
        6,
        Action::EnterFarm(eve.clone(), rust_biguint!(500)),
        Expected::new(
            rust_biguint!(700),
            rust_biguint!(100_000_000_000),
            rust_biguint!(3_500),
        ),
    );
    farm_setup.step(
        10,
        Action::ExitFarm(bob, 2, rust_biguint!(2_000), rust_biguint!(428)),
        Expected::new(
            rust_biguint!(672),
            rust_biguint!(214_285_714_285),
            rust_biguint!(1_500),
        ),
    );
    farm_setup.step(
        13,
        Action::ExitFarm(alice, 1, rust_biguint!(1_000), rust_biguint!(414)),
        Expected::new(
            rust_biguint!(558),
            rust_biguint!(414_285_714_285),
            rust_biguint!(500),
        ),
    );
    farm_setup.step(
        16,
        Action::ExitFarm(eve, 3, rust_biguint!(500), rust_biguint!(457)),
        Expected::new(
            rust_biguint!(401),
            rust_biguint!(1_014_285_714_285),
            rust_biguint!(0),
        ),
    );
}

#[test]
fn test_overview_but_changes_in_per_reward_block() {
    let per_block_reward_amount = rust_biguint!(100);
    let mut farm_setup = FarmRewardsDistrSetup::new(farm::contract_obj, per_block_reward_amount);
    let alice = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    let bob = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    let eve = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    farm_setup.step(
        3,
        Action::EnterFarm(alice.clone(), rust_biguint!(1_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(1_000)),
    );
    farm_setup.step(
        3,
        Action::EnterFarm(bob.clone(), rust_biguint!(2_000)),
        Expected::new(rust_biguint!(400), rust_biguint!(0), rust_biguint!(3_000)),
    );
    farm_setup.step(
        6,
        Action::EnterFarm(eve.clone(), rust_biguint!(500)),
        Expected::new(
            rust_biguint!(700),
            rust_biguint!(100_000_000_000),
            rust_biguint!(3_500),
        ),
    );
    farm_setup.step(
        8,
        Action::RewardPerBlockRateChange(rust_biguint!(50)),
        Expected::new(
            rust_biguint!(900),
            rust_biguint!(157_142_857_142),
            rust_biguint!(3_500),
        ),
    );
    farm_setup.step(
        10,
        Action::ExitFarm(bob, 2, rust_biguint!(2_000), rust_biguint!(371)),
        Expected::new(
            rust_biguint!(629),
            rust_biguint!(185_714_285_713),
            rust_biguint!(1_500),
        ),
    );
    farm_setup.step(
        13,
        Action::ExitFarm(alice, 1, rust_biguint!(1_000), rust_biguint!(285)),
        Expected::new(
            rust_biguint!(494),
            rust_biguint!(285_714_285_713),
            rust_biguint!(500),
        ),
    );
    farm_setup.step(
        16,
        Action::ExitFarm(eve, 3, rust_biguint!(500), rust_biguint!(242)),
        Expected::new(
            rust_biguint!(402),
            rust_biguint!(585_714_285_713),
            rust_biguint!(0),
        ),
    );
}

fn parse_biguint(str: &str) -> RustBigUint {
    let str_without_underscores = str.to_owned().replace('_', "");
    RustBigUint::parse_bytes(str_without_underscores.as_bytes(), 10).unwrap()
}

fn exp18(value: u64) -> RustBigUint {
    value.mul(rust_biguint!(10).pow(18))
}

#[test]
fn test_overview_realistic_numbers() {
    let per_block_reward_amount = exp18(100);
    let mut farm_setup = FarmRewardsDistrSetup::new(farm::contract_obj, per_block_reward_amount);
    let alice = farm_setup.new_address_with_lp_tokens(exp18(5_000));
    let bob = farm_setup.new_address_with_lp_tokens(exp18(5_000));
    let eve = farm_setup.new_address_with_lp_tokens(exp18(5_000));
    farm_setup.step(
        3,
        Action::EnterFarm(alice.clone(), exp18(1_000)),
        Expected::new(exp18(400), rust_biguint!(0), exp18(1_000)),
    );
    farm_setup.step(
        3,
        Action::EnterFarm(bob.clone(), exp18(2_000)),
        Expected::new(exp18(400), rust_biguint!(0), exp18(3_000)),
    );
    farm_setup.step(
        6,
        Action::EnterFarm(eve.clone(), exp18(500)),
        Expected::new(exp18(700), rust_biguint!(100_000_000_000), exp18(3_500)),
    );
    farm_setup.step(
        10,
        Action::ExitFarm(
            bob,
            2,
            exp18(2_000),
            parse_biguint("428_571_428_570_000_000_000"),
        ),
        Expected::new(
            parse_biguint("671_428_571_430_000_000_000"),
            rust_biguint!(214_285_714_285),
            exp18(1_500),
        ),
    );
    farm_setup.step(
        13,
        Action::ExitFarm(
            alice,
            1,
            exp18(1_000),
            parse_biguint("414_285_714_285_000_000_000"),
        ),
        Expected::new(
            parse_biguint("557_142_857_145_000_000_000"),
            rust_biguint!(414_285_714_285),
            exp18(500),
        ),
    );
    farm_setup.step(
        16,
        Action::ExitFarm(
            eve,
            3,
            exp18(500),
            parse_biguint("457_142_857_142_500_000_000"),
        ),
        Expected::new(
            parse_biguint("400_000_000_002_500_000_000"),
            rust_biguint!(1_014_285_714_285),
            exp18(0),
        ),
    );
}

fn exp21(value: u64) -> RustBigUint {
    value.mul(rust_biguint!(10).pow(21))
}

#[test]
fn test_billion_to_trillion() {
    let per_block_reward_amount = exp21(100);
    let mut farm_setup = FarmRewardsDistrSetup::new(farm::contract_obj, per_block_reward_amount);
    let alice = farm_setup.new_address_with_lp_tokens(exp21(5_000));
    let bob = farm_setup.new_address_with_lp_tokens(exp21(5_000));
    let eve = farm_setup.new_address_with_lp_tokens(exp21(5_000));
    farm_setup.step(
        3,
        Action::EnterFarm(alice.clone(), exp21(1_000)),
        Expected::new(exp21(400), rust_biguint!(0), exp21(1_000)),
    );
    farm_setup.step(
        3,
        Action::EnterFarm(bob.clone(), exp21(2_000)),
        Expected::new(exp21(400), rust_biguint!(0), exp21(3_000)),
    );
    farm_setup.step(
        6,
        Action::EnterFarm(eve.clone(), exp21(500)),
        Expected::new(exp21(700), rust_biguint!(100_000_000_000), exp21(3_500)),
    );
    farm_setup.step(
        10,
        Action::ExitFarm(
            bob,
            2,
            exp21(2_000),
            parse_biguint("428_571_428_570_000_000_000_000"),
        ),
        Expected::new(
            parse_biguint("671_428_571_430_000_000_000_000"),
            rust_biguint!(214_285_714_285),
            exp21(1_500),
        ),
    );
    farm_setup.step(
        13,
        Action::ExitFarm(
            alice,
            1,
            exp21(1_000),
            parse_biguint("414_285_714_285_000_000_000_000"),
        ),
        Expected::new(
            parse_biguint("557_142_857_145_000_000_000_000"),
            rust_biguint!(414_285_714_285),
            exp21(500),
        ),
    );
    farm_setup.step(
        16,
        Action::ExitFarm(
            eve,
            3,
            exp21(500),
            parse_biguint("457_142_857_142_500_000_000_000"),
        ),
        Expected::new(
            parse_biguint("400_000_000_002_500_000_000_000"),
            rust_biguint!(1_014_285_714_285),
            exp21(0),
        ),
    );
}

#[test]
fn test_rv_earn_twice() {
    let per_block_reward_amount = rust_biguint!(100);
    let mut farm_setup = FarmRewardsDistrSetup::new(farm::contract_obj, per_block_reward_amount);
    let alice = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    let bob = farm_setup.new_address_with_lp_tokens(rust_biguint!(5_000));
    farm_setup.step(
        1,
        Action::EnterFarm(alice.clone(), rust_biguint!(100)),
        Expected::new(rust_biguint!(200), rust_biguint!(0), rust_biguint!(100)),
    );
    farm_setup.step(
        2,
        Action::EnterFarm(bob.clone(), rust_biguint!(100)),
        Expected::new(
            rust_biguint!(300),
            rust_biguint!(1_000_000_000_000),
            rust_biguint!(200),
        ),
    );
    farm_setup.step(
        9,
        Action::ExitFarm(alice, 1, rust_biguint!(100), rust_biguint!(450)),
        Expected::new(
            rust_biguint!(550),
            rust_biguint!(4_500_000_000_000),
            rust_biguint!(100),
        ),
    );
    farm_setup.step(
        9,
        Action::ExitFarm(bob, 2, rust_biguint!(100), rust_biguint!(350)),
        Expected::new(
            rust_biguint!(200),
            rust_biguint!(4_500_000_000_000),
            rust_biguint!(0),
        ),
    );
}
