use std::{collections::HashMap, str::FromStr};

use crate::prelude::Error;
use rust_decimal::Decimal;
use time::OffsetDateTime;

pub mod ma;
pub mod stddev;

pub const MA_IDX_PRICE: usize = 0;
pub const MA_IDX_QUNATITY: usize = 1;

#[derive(Clone, Copy)]
pub struct KRaw {
    pub time_begin: OffsetDateTime,
    pub time_end: OffsetDateTime,
    pub price_open: Decimal,
    pub price_close: Decimal,
    pub price_high: Decimal,
    pub price_low: Decimal,
    pub quantity: Decimal,
}

#[derive(Clone, Copy)]
pub struct PriceBar {
    pub high: Decimal,
    pub low: Decimal,
    pub mid: Decimal,
}

#[derive(Clone, Copy)]
pub struct KInfo {
    pub raw: KRaw,
    pub body: PriceBar,
    pub full: PriceBar,
    pub quantity: Decimal,
}

#[derive(Clone)]
pub struct KSummary {
    pub info: KInfo,
    pub bases: [HashMap<String, Decimal>; 2],
}

pub trait BaseIndicator:
    Indicator<State = Decimal, Item = KInfo, Value = Decimal> + FromStr<Err = Error>
{
    fn kind(&self) -> usize;
    fn key(&self) -> &str;
}

pub trait Indicator: Sized {
    type State: Clone;
    type Item;
    type Value;

    fn state(&self) -> Option<&Self::State>;
    fn update(&mut self, next: &Self::Item) -> Option<Self::Value>;
    fn calc(&self, next: &Self::Item) -> Option<Self::Value>;
}
