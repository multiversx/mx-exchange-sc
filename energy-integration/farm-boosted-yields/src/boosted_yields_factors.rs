use common_types::Week;
use weekly_rewards_splitting::USER_MAX_CLAIM_WEEKS;

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

static INVALID_CONFIG_WEEK_ERR_MSG: &[u8] = b"Invalid config week";
static NO_CONFIG_ERR_MSG: &[u8] = b"No config";
const BOOSTED_YIELDS_FACTORS_ARRAY_LEN: usize = USER_MAX_CLAIM_WEEKS + 1;

#[derive(TypeAbi, TopEncode, TopDecode, NestedEncode, NestedDecode, Clone, PartialEq, Debug)]
pub struct BoostedYieldsFactors<M: ManagedTypeApi> {
    pub max_rewards_factor: BigUint<M>,
    pub user_rewards_energy_const: BigUint<M>,
    pub user_rewards_farm_const: BigUint<M>,
    pub min_energy_amount: BigUint<M>,
    pub min_farm_amount: BigUint<M>,
}

#[derive(TypeAbi, TopEncode, TopDecode, Clone, PartialEq, Debug)]
pub struct BoostedYieldsConfig<M: ManagedTypeApi> {
    last_update_week: Week,
    factors_per_week: ArrayVec<BoostedYieldsFactors<M>, BOOSTED_YIELDS_FACTORS_ARRAY_LEN>,
}

impl<M: ManagedTypeApi> BoostedYieldsConfig<M> {
    pub fn new(current_week: Week, factors: BoostedYieldsFactors<M>) -> Self {
        let mut factors_per_week = ArrayVec::new();
        for _ in 0..BOOSTED_YIELDS_FACTORS_ARRAY_LEN {
            unsafe {
                factors_per_week.push_unchecked(factors.clone());
            }
        }

        BoostedYieldsConfig {
            factors_per_week,
            last_update_week: current_week,
        }
    }

    pub fn update(
        &mut self,
        current_week: Week,
        opt_new_boost_factors: Option<BoostedYieldsFactors<M>>,
    ) {
        if current_week < self.last_update_week {
            M::error_api_impl().signal_error(INVALID_CONFIG_WEEK_ERR_MSG);
        }

        let mut week_diff = current_week - self.last_update_week;
        week_diff = core::cmp::min(week_diff, BOOSTED_YIELDS_FACTORS_ARRAY_LEN);
        if week_diff == 0 {
            if let Some(new_boost_factors) = opt_new_boost_factors {
                self.factors_per_week[BOOSTED_YIELDS_FACTORS_ARRAY_LEN - 1] = new_boost_factors;
            }

            return;
        }

        // shift left by week diff
        // only change the current_week entry to the latest, keep the rest to the current config
        let current_last_factors =
            self.factors_per_week[BOOSTED_YIELDS_FACTORS_ARRAY_LEN - 1].clone();
        let _ = self.factors_per_week.drain(0..week_diff);
        for _ in 0..(week_diff - 1) {
            unsafe {
                self.factors_per_week
                    .push_unchecked(current_last_factors.clone());
            }
        }

        let latest_config = opt_new_boost_factors.unwrap_or(current_last_factors);
        unsafe {
            self.factors_per_week.push_unchecked(latest_config);
        }

        self.last_update_week = current_week;
    }

    pub fn get_factors_for_week(&self, week: Week) -> &BoostedYieldsFactors<M> {
        if week >= self.last_update_week {
            M::error_api_impl().signal_error(INVALID_CONFIG_WEEK_ERR_MSG);
        }

        let offset = self.last_update_week - week;
        if offset >= BOOSTED_YIELDS_FACTORS_ARRAY_LEN {
            M::error_api_impl().signal_error(INVALID_CONFIG_WEEK_ERR_MSG);
        }

        let last_item_index = BOOSTED_YIELDS_FACTORS_ARRAY_LEN - 1;
        &self.factors_per_week[last_item_index - offset]
    }

    pub fn get_latest_factors(&self) -> BoostedYieldsFactors<M> {
        self.factors_per_week[BOOSTED_YIELDS_FACTORS_ARRAY_LEN - 1].clone()
    }
}

#[multiversx_sc::module]
pub trait BoostedYieldsFactorsModule:
    permissions_module::PermissionsModule + week_timekeeping::WeekTimekeepingModule
{
    #[endpoint(setBoostedYieldsFactors)]
    fn set_boosted_yields_factors(
        &self,
        max_rewards_factor: BigUint,
        user_rewards_energy_const: BigUint,
        user_rewards_farm_const: BigUint,
        min_energy_amount: BigUint,
        min_farm_amount: BigUint,
    ) {
        self.require_caller_has_admin_permissions();
        require!(
            min_energy_amount > 0 && min_farm_amount > 0,
            "Min amounts must be greater than 0"
        );

        let factors = BoostedYieldsFactors {
            max_rewards_factor,
            user_rewards_energy_const,
            user_rewards_farm_const,
            min_energy_amount,
            min_farm_amount,
        };

        let current_week = self.get_current_week();
        let config_mapper = self.boosted_yields_config();
        if !config_mapper.is_empty() {
            config_mapper.update(|config| {
                config.update(current_week, Some(factors));
            });
        } else {
            let config = BoostedYieldsConfig::new(current_week, factors);
            config_mapper.set(&config);
        }
    }

    fn get_updated_boosted_yields_config(&self) -> BoostedYieldsConfig<Self::Api> {
        let opt_config = self.try_get_boosted_yields_config();
        opt_config.unwrap_or_else(|| sc_panic!(NO_CONFIG_ERR_MSG))
    }

    fn try_get_boosted_yields_config(&self) -> Option<BoostedYieldsConfig<Self::Api>> {
        let mapper = self.boosted_yields_config();
        if mapper.is_empty() {
            return None;
        }

        let current_week = self.get_current_week();
        let mut config = self.boosted_yields_config().get();
        config.update(current_week, None);

        Some(config)
    }

    fn update_boosted_yields_config(&self) {
        let updated_config = self.get_updated_boosted_yields_config();
        self.boosted_yields_config().set(&updated_config);
    }

    #[view(getBoostedYieldsFactors)]
    fn get_boosted_yields_factors(&self) -> BoostedYieldsFactors<Self::Api> {
        let config = self.boosted_yields_config().get();
        config.get_latest_factors()
    }

    #[storage_mapper("boostedYieldsConfig")]
    fn boosted_yields_config(&self) -> SingleValueMapper<BoostedYieldsConfig<Self::Api>>;
}
