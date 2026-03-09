use rust_decimal::Decimal;
use time::OffsetDateTime;

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
