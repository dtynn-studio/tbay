//! 数据转换：Binance JSON 响应 → 内部 K 类型

use serde::Deserialize;

// =============================================================================
// REST API
// =============================================================================

/// Binance REST API 返回的 K线数据（数组格式）
/// API: GET /fapi/v1/klines
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KlineSummary {
    pub open_time: i64,
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
    pub close_time: i64,
    pub number_of_trades: i64,
}

/// KlineSummaries 包装类型
#[derive(Debug)]
pub enum KlineSummaries {
    AllKlineSummaries(Vec<KlineSummary>),
}

// =============================================================================
// WebSocket
// =============================================================================

/// WebSocket K线事件
/// https://binance-docs.github.io/apidocs/futures/en/#kline-candlestick-streams
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct KlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "k")]
    pub kline: Kline,
}

/// WebSocket 连续合约 K线事件
/// https://binance-docs.github.io/apidocs/futures/en/#continuous-contract-kline-candlestick-streams
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContinuousKlineEvent {
    #[serde(rename = "e")]
    pub event_type: String,

    #[serde(rename = "E")]
    pub event_time: u64,

    #[serde(rename = "ps")]
    pub pair: String,

    #[serde(rename = "ct")]
    pub contract_type: String,

    #[serde(rename = "k")]
    pub kline: ContinuousKline,
}

/// WebSocket K线数据
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Kline {
    #[serde(rename = "t")]
    pub open_time: i64,

    #[serde(rename = "T")]
    pub close_time: i64,

    #[serde(rename = "s")]
    pub symbol: String,

    #[serde(rename = "i")]
    pub interval: String,

    #[serde(rename = "o")]
    pub open: String,

    #[serde(rename = "c")]
    pub close: String,

    #[serde(rename = "h")]
    pub high: String,

    #[serde(rename = "l")]
    pub low: String,

    #[serde(rename = "v")]
    pub volume: String,

    #[serde(rename = "n")]
    pub number_of_trades: i64,

    #[serde(rename = "x")]
    pub is_final_bar: bool,
}

/// WebSocket 连续合约 K线数据
#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContinuousKline {
    #[serde(rename = "t")]
    pub start_time: i64,

    #[serde(rename = "T")]
    pub end_time: i64,

    #[serde(rename = "i")]
    pub interval: String,

    #[serde(rename = "o")]
    pub open: String,

    #[serde(rename = "c")]
    pub close: String,

    #[serde(rename = "h")]
    pub high: String,

    #[serde(rename = "l")]
    pub low: String,

    #[serde(rename = "v")]
    pub volume: String,

    #[serde(rename = "n")]
    pub number_of_trades: i64,

    #[serde(rename = "x")]
    pub is_final_bar: bool,
}
