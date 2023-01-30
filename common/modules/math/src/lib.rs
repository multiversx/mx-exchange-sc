#![no_std]

multiversx_sc::imports!();
multiversx_sc::derive_imports!();

/// out = (min_out * (max_in - current_in) + max_out * (current_in - min_in)) / (max_in - min_in)
/// https://en.wikipedia.org/wiki/Linear_interpolation
pub fn linear_interpolation<M, T>(min_in: T, max_in: T, current_in: T, min_out: T, max_out: T) -> T
where
    M: ManagedTypeApi,
    T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + PartialOrd + Clone,
{
    if current_in < min_in || current_in > max_in {
        M::error_api_impl().signal_error(b"Invalid values");
    }

    let min_out_weighted = min_out * (max_in.clone() - current_in.clone());
    let max_out_weighted = max_out * (current_in - min_in.clone());
    let in_diff = max_in - min_in;

    (min_out_weighted + max_out_weighted) / in_diff
}

pub fn weighted_average<T>(first_value: T, first_weight: T, second_value: T, second_weight: T) -> T
where
    T: Add<Output = T> + Mul<Output = T> + Div<Output = T> + Clone,
{
    let weight_sum = first_weight.clone() + second_weight.clone();
    let weighted_sum = first_value * first_weight + second_value * second_weight;
    weighted_sum / weight_sum
}

pub fn weighted_average_round_up<T>(
    first_value: T,
    first_weight: T,
    second_value: T,
    second_weight: T,
) -> T
where
    T: Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + Clone + From<u32>,
{
    let weight_sum = first_weight.clone() + second_weight.clone();
    let weighted_sum = first_value * first_weight + second_value * second_weight;
    (weighted_sum + weight_sum.clone() - T::from(1u32)) / weight_sum
}

/// computes first_value - second_value, returning 0 if the result would be negative
pub fn safe_sub<T>(first_value: T, second_value: T) -> T
where
    T: Sub<Output = T> + PartialOrd + Default,
{
    if first_value > second_value {
        first_value - second_value
    } else {
        T::default()
    }
}
