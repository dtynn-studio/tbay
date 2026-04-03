# Rate Monitor Design

## Overview

The `rate` monitor calculates the ratio between a current value (`val`) and a base indicator (`base`), alerting when the ratio meets or exceeds a threshold.

## Configuration

**Format:** `rate:val_kind,calc_kind,period,mode,threshold`

**Example:** `rate:price_close,ema,20,abs,1.5`

| Parameter | Type | Description |
|-----------|------|-------------|
| `val_kind` | `ValKind` | `price_close` or `quantity` |
| `calc_kind` | `CalcKind` | Indicator type: `ema`, `sma`, etc. |
| `period` | `u32` | Indicator period (e.g., 20) |
| `mode` | `RateMode` | `abs` or `dif` |
| `threshold` | `f64` | Alert threshold (rate >= threshold) |

## Rate Calculation

- **abs mode:** `rate = val / base` (always positive)
- **dif mode:** `rate = (val - base) / base` (can be positive or negative)

## Output Format

- **abs mode:** `(C/EM20):1.523x`
- **dif mode:** `(C/EM20):+7.15%` or `(C/EM20):-15.32%`

Format details:
- ValKind prefix: `C` for price_close, `Q` for quantity
- Base indicator shorthand: `EM20` (EMA 20), `SM50` (SMA 50), etc.
- Sign for dif mode only: `+` or `-`
- abs shows `x` suffix, dif shows `%` suffix

## Architecture

### File Structure
New file: `src/monitor/rate.rs`

### Components

1. **`ValKind` enum** - `PriceClose`, `Quantity`
2. **`RateMode` enum** - `Abs`, `Dif`
3. **`RateArgs` struct** - Parses and holds configuration
4. **`Rate` struct** - Monitor implementation

### Value Retrieval

- `val` accessed directly from `kctx.info.raw.price_close` or `kctx.info.raw.quantity`
- `base` retrieved via `kctx.get_val(&base_key)` using `BaseExtractorArgs::new()` pattern

### Dependencies

```rust
fn deps(&self) -> Vec<&str> {
    vec![&self.base_key]
}
```

Only the base indicator is declared as dependency since `val_kind` values come directly from `kctx.info.raw`.

### Alert Behavior

- Alert when `rate >= threshold`
- Division by zero returns `None` (no alert, no error)
- Non-finalized K lines: temp state + alerts
- Finalized K lines: perm state only

## Implementation Pattern

Follows existing monitor patterns (`touch.rs`, `cross.rs`):
- `Args` trait with `build()` method
- `impl_builder!` macro for Builder
- Standard `apply()` pattern for finalized/non-finalized K lines
