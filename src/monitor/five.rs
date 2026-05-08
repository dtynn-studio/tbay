use std::borrow::Cow;

use crossterm::style::Stylize;
use scanf::sscanf;

use crate::{
    impl_builder,
    indicator::{
        base::{BaseExtractor, BaseExtractorArgs, CalcKind, ExtractKind},
        cross::{MaCross, MaCrossArgs},
        position2::{Pos, Position2, Position2Args},
        shadow::{Shadow, ShadowArgs},
    },
    monitor::{Msg, State, alert::AlertManager},
    prelude::*,
    util::ring_buffer::RingBuffer,
};

// format: s:{threshold}
// example: s:0.6
#[derive(Clone, Copy)]
pub struct FiveShadowArgs {
    pub threshold: f64,
}

impl FromStr for FiveShadowArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut threshold = 0.0f64;

        sscanf!(s, "s:{threshold}").with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse five shadow args"),
        })?;

        Ok(Self { threshold })
    }
}

impl FiveShadowArgs {
    fn key(&self) -> String {
        format!("s:{}", self.threshold)
    }
}

// format: q:{calc_kind},{ma_period},{burst_threshold},{continuous_threshold},{continuous_duration}
// example: q:sma,10,3.0,0.8,5
#[derive(Clone, Copy)]
pub struct FiveQtyArgs {
    pub calc_kind: CalcKind,
    pub ma_period: usize,
    pub burst_threshold: f64,
    pub continuous_threshold: f64,
    pub continuous_duration: usize,
}

impl FromStr for FiveQtyArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut ma_period = 0usize;
        let mut burst_threshold = 0.0f64;
        let mut continuous_threshold = 0.0f64;
        let mut continuous_duration = 0usize;

        sscanf!(
            s,
            "q:{calc_kind_str},{ma_period},{burst_threshold},{continuous_threshold},{continuous_duration}"
        )
        .with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse five qty args"),
        })?;

        let calc_kind = calc_kind_str.parse()?;

        if ma_period == 0 {
            return Err(ma_period.unexpected("five qty ma period"));
        }

        if continuous_duration == 0 {
            return Err(
                continuous_duration.unexpected("five qty continuous duration")
            );
        }

        Ok(Self {
            calc_kind,
            ma_period,
            burst_threshold,
            continuous_threshold,
            continuous_duration,
        })
    }
}

impl FiveQtyArgs {
    fn key(&self) -> String {
        format!(
            "q:{},{},{},{},{}",
            self.calc_kind.as_str(),
            self.ma_period,
            self.burst_threshold,
            self.continuous_threshold,
            self.continuous_duration
        )
    }
}

// format: p:{calc_kind},{fast_period},{slow_period},{hold_period},{hold_duration}
// example: p:ema,5,20,20,3
#[derive(Clone, Copy)]
pub struct FivePriceArgs {
    pub calc_kind: CalcKind,
    pub fast_period: usize,
    pub slow_period: usize,
    pub hold_period: usize,
    pub hold_duration: usize,
}

impl FromStr for FivePriceArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut calc_kind_str = String::new();
        let mut fast_period = 0usize;
        let mut slow_period = 0usize;
        let mut hold_period = 0usize;
        let mut hold_duration = 0usize;

        sscanf!(
            s,
            "p:{calc_kind_str},{fast_period},{slow_period},{hold_period},{hold_duration}"
        )
        .with_context(|_| ParseCtx {
            raw: s.to_owned(),
            usage: Cow::from("parse five price args"),
        })?;

        let calc_kind = calc_kind_str.parse()?;

        if fast_period == 0 {
            return Err(fast_period.unexpected("five price fast period"));
        }

        if slow_period == 0 {
            return Err(slow_period.unexpected("five price slow period"));
        }

        if hold_period == 0 {
            return Err(hold_period.unexpected("five price hold period"));
        }

        if hold_duration == 0 {
            return Err(hold_duration.unexpected("five price hold duration"));
        }

        Ok(Self {
            calc_kind,
            fast_period,
            slow_period,
            hold_period,
            hold_duration,
        })
    }
}

impl FivePriceArgs {
    fn key(&self) -> String {
        format!(
            "p:{},{},{},{},{}",
            self.calc_kind.as_str(),
            self.fast_period,
            self.slow_period,
            self.hold_period,
            self.hold_duration
        )
    }
}

// format: five:{shadow_args_str}/{qty_args_str}/{price_args_str}
// example: five:s:0.6/q:sma,10,3.0,0.8,5/p:ema,5,20,20,3
#[derive(Clone, Copy)]
pub struct FiveArgs {
    pub lookback: usize,
    pub shadow: FiveShadowArgs,
    pub qty: FiveQtyArgs,
    pub price: FivePriceArgs,
}

impl FromStr for FiveArgs {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut lookback = 0usize;
        let mut shadow_str = String::new();
        let mut qty_str = String::new();
        let mut price_str = String::new();

        sscanf!(s, "five:{lookback}/{shadow_str}/{qty_str}/{price_str}")
            .with_context(|_| ParseCtx {
                raw: s.to_owned(),
                usage: Cow::from("parse five args"),
            })?;

        if lookback == 0 {
            return Err(lookback.unexpected("five lookback"));
        }

        let shadow = shadow_str.parse()?;
        let qty = qty_str.parse()?;
        let price = price_str.parse()?;

        Ok(Self {
            lookback,
            shadow,
            qty,
            price,
        })
    }
}

impl Args for FiveArgs {
    type Type = (usize, FiveShadowArgs, FiveQtyArgs, FivePriceArgs);
    type Target = Five;

    fn new(args: Self::Type) -> Self {
        Self {
            lookback: args.0,
            shadow: args.1,
            qty: args.2,
            price: args.3,
        }
    }

    fn key(&self) -> String {
        format!(
            "five:{}/{}/{}/{}",
            self.lookback,
            self.shadow.key(),
            self.qty.key(),
            self.price.key()
        )
    }

    fn build(self) -> Result<Self::Target> {
        let key = self.key();
        let shadow_key = ShadowArgs::new(self.shadow.threshold).key();
        let qty_ma_key = BaseExtractorArgs::new((
            ExtractKind::Qty,
            self.qty.calc_kind,
            self.qty.ma_period,
        ))
        .key();
        let price_cross_key = MaCrossArgs::new((
            ExtractKind::PriceClose,
            self.price.calc_kind,
            self.price.fast_period,
            self.price.slow_period,
        ))
        .key();

        let price_position_key =
            Position2Args::new((self.price.calc_kind, self.price.hold_period))
                .key();

        let qty_burst_threshold = Decimal::from_f64(self.qty.burst_threshold)
            .required("qty burst threshold")?;

        let qty_continuous_threshold =
            Decimal::from_f64(self.qty.continuous_threshold)
                .required("qty continuous threshold")?;

        Ok(Five {
            args: self,
            key,
            shadow_key,
            qty_ma_key,
            price_cross_key,
            price_position_key,
            qty_burst_threshold,
            qty_continuous_threshold,
            lookback_states: RingBuffer::new(self.lookback),
            prev_state_bits: 0,
            state: Default::default(),
            alert: Default::default(),
        })
    }
}

impl_builder!(FiveBuilder: FiveArgs => Five);

#[derive(Clone, Copy)]
struct QtyBurst {
    abs: Decimal,
    ratio: Decimal,
}

#[derive(Clone, Copy, Default)]
struct KState {
    shadow: Option<<Shadow as Indicator>::Output>,
    qty_burst: Option<QtyBurst>,
    qty_continuous: Option<usize>,
    price_ma_cross: Option<bool>,
    price_hold: Option<<Position2 as Indicator>::Output>,
}

impl KState {
    fn merge(&mut self, other: &KState) {
        // 越近的影线越优先处理
        if let Some(s) = other.shadow.as_ref().copied() {
            self.shadow.replace(s);
        }

        // 越近的交易量爆发
        if let Some(b) = other.qty_burst.as_ref().copied() {
            self.qty_burst.replace(b);
        }

        // 交易量持续时间越长越优先
        if let Some(other_d) = other.qty_continuous {
            match self.qty_continuous {
                None => {
                    self.qty_continuous.replace(other_d);
                }

                Some(prev_d) => {
                    if other_d > prev_d {
                        self.qty_continuous.replace(other_d);
                    }
                }
            }
        }

        // 越近的均线交叉越优先
        if let Some(cross) = other.price_ma_cross {
            self.price_ma_cross.replace(cross);
        }

        // 越近的价格运行越优先
        if let Some(hold) = other.price_hold.as_ref().copied() {
            self.price_hold.replace(hold);
        }
    }
}

pub struct Five {
    args: FiveArgs,
    key: String,

    // deps: indicator keys
    shadow_key: String,         // use ::indicator::shadow
    qty_ma_key: String,         // use ::indicator::base::BaseExtractor for qty
    price_cross_key: String,    // use ::indicator::cross
    price_position_key: String, // use ::indicator::position2

    // decimal args
    qty_burst_threshold: Decimal,
    qty_continuous_threshold: Decimal,

    // k states
    lookback_states: RingBuffer<KState>,
    prev_state_bits: u8,

    // mgr
    state: State,
    alert: AlertManager,
}

impl Five {
    fn gen_k_state(&self, kctx: &KCtx) -> KState {
        let mut state = KState::default();

        // get shadow
        if let Some(shadow) = kctx
            .get_val::<<Shadow as Indicator>::Output>(&self.shadow_key)
            .copied()
        {
            state.shadow.replace(shadow);
        }

        // calc qty_busrt ad qty_continuous
        if let Some(qty_ma_base) = kctx
            .get_val::<<BaseExtractor as Indicator>::Output>(&self.qty_ma_key)
            .copied()
            && !qty_ma_base.is_zero()
        {
            let qty_ratio = kctx.info.raw.quantity / qty_ma_base;
            if qty_ratio >= self.qty_burst_threshold {
                state.qty_burst.replace(QtyBurst {
                    abs: kctx.info.raw.quantity,
                    ratio: qty_ratio,
                });
            };

            if qty_ratio >= self.qty_continuous_threshold {
                if let Some(prev_duration) =
                    self.lookback_states.last().and_then(|s| s.qty_continuous)
                {
                    state.qty_continuous.replace(prev_duration + 1);
                } else {
                    state.qty_continuous.replace(1);
                }
            }
        }

        if let Some(cross_direction) = kctx
            .get_val::<<MaCross as Indicator>::Output>(&self.price_cross_key)
            .and_then(|c| c.cross)
        {
            state.price_ma_cross.replace(cross_direction);
        }

        if let Some(hold) = kctx
            .get_val::<<Position2 as Indicator>::Output>(
                &self.price_position_key,
            )
            .copied()
        {
            state.price_hold.replace(hold);
        }

        state
    }

    fn gen_k_state_bits(&self, k_state: &KState) -> u8 {
        let mut bits = 0;

        if k_state.shadow.is_some() {
            bits |= 1;
        }

        if k_state.qty_burst.is_some() {
            bits |= 0b10;
        }

        if let Some(d) = k_state.qty_continuous
            && d >= self.args.qty.continuous_duration
        {
            bits |= 0b100;
        }

        if k_state.price_ma_cross.is_some() {
            bits |= 0b1000;
        }

        if let Some(hold) = k_state.price_hold.as_ref()
            && hold.pos != Pos::Chaos
            && hold.periods >= self.args.price.hold_duration
        {
            bits |= 0b10000;
        }

        bits
    }

    fn gen_k_state_msg(&self, kctx: &KCtx, k_state: &KState) -> Option<Msg> {
        let mut normal_pieces = Vec::with_capacity(5);
        let mut tty_pieces = Vec::with_capacity(5);

        if let Some(s) = k_state.shadow {
            let (flag, color) = if s.is_above {
                ("┴", kctx.colors.down)
            } else {
                ("┬", kctx.colors.up)
            };

            let ratio = s.ratio.round_dp(2);
            let abs = s.abs.round_dp(2);

            normal_pieces.push(format!("{flag}{ratio}({abs})"));
            tty_pieces
                .push(format!("{flag}{ratio}({abs})").with(color).to_string());
        }

        if let Some(qty_burst) = k_state.qty_burst {
            let ratio = qty_burst.ratio.round_dp(2);
            let abs = qty_burst.abs.round_dp(2);
            let msg = format!("<{ratio}({abs})");
            normal_pieces.push(msg.clone());
            tty_pieces.push(msg.with(kctx.colors.up).to_string());
        }

        if let Some(d) = k_state.qty_continuous
            && d >= self.args.qty.continuous_duration
        {
            let msg = format!("={d}");
            normal_pieces.push(msg.clone());
            tty_pieces.push(msg);
        }

        if let Some(cross) = k_state.price_ma_cross {
            let (flag, color) = if cross {
                ("↗", kctx.colors.up)
            } else {
                ("↘", kctx.colors.down)
            };

            normal_pieces.push(flag.to_string());
            tty_pieces.push(flag.with(color).to_string());
        }

        if let Some(hold) = k_state.price_hold
            && hold.periods >= self.args.price.hold_duration
        {
            let (flag, color) = match hold.pos {
                Pos::Above => ("▲", kctx.colors.up),
                Pos::Below => ("▼", kctx.colors.down),
                Pos::Chaos => ("~", kctx.colors.normal),
            };

            let msg = format!("{flag}{}", hold.periods);
            normal_pieces.push(msg.clone());
            tty_pieces.push(msg.with(color).to_string());
        }

        if normal_pieces.is_empty() {
            return None;
        }

        let normal = format!("V:{}", normal_pieces.join("|"));
        let tty = format!("V:{}", tty_pieces.join("|"));

        Some(Msg { normal, tty })
    }
}

impl Monitor for Five {
    fn key(&self) -> &str {
        &self.key
    }

    fn deps(&self) -> Vec<&str> {
        vec![
            &self.shadow_key,
            &self.qty_ma_key,
            &self.price_cross_key,
            &self.price_position_key,
        ]
    }

    fn apply(&mut self, kctx: &KCtx) {
        let t = kctx.info.t();
        let latest_state = self.gen_k_state(kctx);

        let mut merged_state = KState::default();

        let mut skip_n = 0;
        if self.lookback_states.is_full() {
            skip_n = 1;
        }

        for st in self
            .lookback_states
            .all()
            .skip(skip_n)
            .chain([&latest_state])
        {
            merged_state.merge(st);
        }

        let merged_state_msg =
            self.gen_k_state_msg(kctx, &merged_state).map(|m| (t, m));

        if kctx.info.raw.finalized {
            self.lookback_states.update(latest_state);
        }

        self.state.clear();
        let state_bits = self.gen_k_state_bits(&merged_state);
        if state_bits.count_ones() >= 3 {
            if kctx.info.raw.finalized {
                if state_bits != self.prev_state_bits
                    && let Some((t, m)) = merged_state_msg.as_ref()
                {
                    self.prev_state_bits = state_bits;
                    self.alert.add(*t, m.clone());
                    self.lookback_states.reset();
                }

                self.state.perm = merged_state_msg;
            } else {
                self.state.temp = merged_state_msg;
            }
        }
    }

    fn state(&self) -> &State {
        &self.state
    }

    fn take_alerts(&mut self) -> Vec<(OffsetDateTime, Msg)> {
        self.alert.take()
    }

    fn terminated(&self) -> bool {
        false
    }

    fn is_once(&self) -> bool {
        false
    }
}
