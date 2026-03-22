//! 数据转换：Binance JSON 响应 → 内部 K 类型

use serde::Deserialize;

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
