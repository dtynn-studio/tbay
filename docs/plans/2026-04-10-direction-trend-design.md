# K线方向趋势标定设计

## 目标

基于 `KInfo` 所能提供的信息，对单根 K 线的方向趋势进行标定。

## 输出

三种趋势状态：

| 趋势 | 说明 |
|------|------|
| `Trend::Up` | 向上趋势 |
| `Trend::Down` | 向下趋势 |
| `Trend::Unknown` | 不明趋势 |

## 核心概念

| 概念 | 定义 |
|------|------|
| `body_height` | `body.high - body.low`（实体高度） |
| `full_height` | `full.high - full.low`（整体高度） |
| `upper_shadow` | `shadow.above`（上影线高度） |
| `lower_shadow` | `shadow.below`（下影线高度） |

比例计算：
- `body_ratio = body_height / full_height`
- `upper_ratio = upper_shadow / full_height`
- `lower_ratio = lower_shadow / full_height`

## 判定逻辑

```
1. body_ratio >= threshold → Trend::{Up|Down}（由 direction 决定）
2. body_ratio <  threshold 且 upper_ratio >= threshold → Trend::Down
3. body_ratio <  threshold 且 lower_ratio >= threshold → Trend::Up
4. 都不满足 → Trend::Unknown
```

**优先级**：明确实体 > 影线主导 > 不明

## 接口设计

```rust
// 位置：src/k.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    Up,
    Down,
    Unknown,
}

impl KInfo {
    pub fn trend(&self, threshold: Option<Decimal>) -> Trend {
        let threshold = threshold.unwrap_or(Decimal::TWO / Decimal::from(3));

        let full_height = self.full.high - self.full.low;
        if full_height.is_zero() {
            return Trend::Unknown;
        }

        let body_height = self.body.high - self.body.low;
        let upper_shadow = self.shadow.above;
        let lower_shadow = self.shadow.below;

        let body_ratio = body_height / full_height;
        let upper_ratio = upper_shadow / full_height;
        let lower_ratio = lower_shadow / full_height;

        if body_ratio >= threshold {
            match self.direction {
                Some(true) => Trend::Up,
                Some(false) => Trend::Down,
                None => Trend::Unknown,
            }
        } else if upper_ratio >= threshold {
            Trend::Down
        } else if lower_ratio >= threshold {
            Trend::Up
        } else {
            Trend::Unknown
        }
    }
}
```

## 参数说明

| 参数 | 类型 | 说明 |
|------|------|------|
| `threshold` | `Option<Decimal>` | 判定阈值，默认 `2/3`（约 66.67%） |

## 边界情况

| 情况 | 处理 |
|------|------|
| `full_height == 0` | 返回 `Unknown`（价格无波动，非法 K 线） |
| `direction == None` 且 `body_ratio >= threshold` | 返回 `Unknown`（理论上不会发生） |

## 设计决策

1. **使用 height 而非 width**：避免与"K线宽度"概念混淆
2. **统一阈值**：body_ratio 和影线占比使用相同的阈值，简化逻辑
3. **明确实体优先**：当 body_ratio 足够高时，直接以 direction 为准
4. **影线主导作为补充**：当 body 不够强时，影线方向接管判定
