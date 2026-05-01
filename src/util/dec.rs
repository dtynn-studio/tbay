use rust_decimal::Result;

use crate::prelude::*;

const UNITS: &[(Result<Decimal>, &str)] = &[
    (Decimal::try_from_i128_with_scale(1_000_000_000, 0), "b"),
    (Decimal::try_from_i128_with_scale(1_000_000, 0), "m"),
    (Decimal::try_from_i128_with_scale(1_000, 0), "k"),
    (Decimal::try_from_i128_with_scale(1_00, 0), "h"),
];

pub fn format_decimal(val: Decimal, digits: u32) -> String {
    let mut unit = "";
    let mut num = val.round_dp(digits);
    for (res, u) in UNITS {
        if let Ok(base) = res
            && val >= *base
        {
            num = (val / base).round_dp(digits);
            unit = u;
            break;
        }
    }

    format!("{num}{unit}")
}
