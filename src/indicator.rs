use std::collections::HashMap;

use rust_decimal::Decimal;
use time::OffsetDateTime;

pub const MA_IDX_PRICE: usize = 0;
pub const MA_IDX_QUNATITY: usize = 1;

#[derive(Clone, Copy)]
pub struct PriceBar {
    pub high: Decimal,
    pub low: Decimal,
    pub mid: Decimal,
}

#[derive(Clone, Copy)]
pub struct KBase {
    pub t_begin: OffsetDateTime,
    pub t_end: OffsetDateTime,
    pub price_open: Decimal,
    pub price_close: Decimal,
    pub body: PriceBar,
    pub full: PriceBar,
    pub quantity: Decimal,
}

#[derive(Clone)]
pub struct KSummary {
    pub base: KBase,
    pub mas: [HashMap<usize, Decimal>; 2],
}

pub trait BaseIndicator {
    fn ma_idx(&self) -> usize;
    fn ma_key(&self) -> usize;
    fn update(&mut self) -> Decimal;
    fn calc(&self) -> Decimal;
}
