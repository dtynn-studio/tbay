use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap},
    time::Duration,
};

use tracing::{debug, warn_span};

use crate::{
    config::{Config, Interval, Pair},
    event::K,
    indicator, monitor,
    prelude::*,
};

pub type HubIndicator = Box<dyn Indicator<Output = Box<dyn Any>>>;
pub type HubIndicatorBuilder = Box<dyn Builder<Target = HubIndicator>>;
pub type HubMonitor = Box<dyn Monitor>;
pub type HubMonitorBuilder = Box<dyn Builder<Target = HubMonitor>>;

pub struct HubItem {
    pub symbol: String,
    pub indicators: BTreeMap<Duration, Vec<HubIndicator>>,
    pub monitors: BTreeMap<Duration, Vec<HubMonitor>>,
}

pub struct Hub {
    indicator_builders: HashMap<TypeId, HubIndicatorBuilder>,
    monitor_builders: HashMap<TypeId, HubMonitorBuilder>,
    items: Vec<HubItem>,
}

impl Default for Hub {
    fn default() -> Self {
        let mut hub = Hub {
            indicator_builders: Default::default(),
            monitor_builders: Default::default(),
            items: Default::default(),
        };

        // indicator builders
        hub.register_indicator_builder(indicator::base::BaseExtractorBuilder);
        hub.register_indicator_builder(
            indicator::bollinger::BollingerBandBuilder,
        );
        hub.register_indicator_builder(indicator::macd::MacdBuilder);
        hub.register_indicator_builder(indicator::cross::MaCrossBuilder);
        hub.register_indicator_builder(indicator::distance::DistanceBuilder);
        hub.register_indicator_builder(indicator::position::PositionBuilder);

        // monitor builders
        hub.register_monitor_builder(monitor::cross::CrossBuilder);
        hub.register_monitor_builder(monitor::touch::TouchBuilder);
        hub.register_monitor_builder(monitor::hold::HoldBuilder);

        hub
    }
}

impl Hub {
    pub fn calc(&self, k: K) -> Option<Vec<String>> {
        let indicators = self
            .items
            .iter()
            .find(|item| item.symbol == k.symbol)
            .and_then(|item| item.indicators.get(&k.interval))?;

        let mut kctx = KCtx::from(k.raw);

        for indicator in indicators {
            if let Some(val) = indicator.calc(&kctx) {
                kctx.set_val(indicator.key(), val);
            }
        }

        let monitors = self
            .items
            .iter()
            .find(|item| item.symbol == k.symbol)
            .and_then(|item| item.monitors.get(&k.interval))?;

        let mut events = vec![];
        for monitor in monitors {
            if let Some(msg) = monitor.calc(&kctx) {
                events.push(msg);
            }
        }

        Some(events)
    }

    pub fn update(&mut self, k: K) -> Option<Vec<String>> {
        let indicators = self
            .items
            .iter_mut()
            .find(|item| item.symbol == k.symbol)
            .and_then(|item| item.indicators.get_mut(&k.interval))?;

        let mut kctx = KCtx::from(k.raw);

        for indicator in indicators {
            if let Some(val) = indicator.update(&kctx) {
                kctx.set_val(indicator.key(), val);
            }
        }

        let monitors = self
            .items
            .iter_mut()
            .find(|item| item.symbol == k.symbol)
            .and_then(|item| item.monitors.get_mut(&k.interval))?;

        let mut events = vec![];
        for monitor in monitors {
            if let Some(msg) = monitor.update(&kctx) {
                events.push(msg);
            }
        }

        Some(events)
    }

    fn has_indicator(
        &self,
        symbol: &str,
        interval: Duration,
        key: &str,
    ) -> bool {
        let Some(item) = self.items.iter().find(|item| item.symbol == symbol)
        else {
            return false;
        };

        let Some(indicators) = item.indicators.get(&interval) else {
            return false;
        };

        indicators.iter().any(|i| i.key() == key)
    }

    fn has_monitor(&self, symbol: &str, interval: Duration, key: &str) -> bool {
        let Some(item) = self.items.iter().find(|item| item.symbol == symbol)
        else {
            return false;
        };

        let Some(monitors) = item.monitors.get(&interval) else {
            return false;
        };

        monitors.iter().any(|i| i.key() == key)
    }

    pub fn register_indicator_builder<
        B: Builder<Target: Indicator> + 'static,
    >(
        &mut self,
        builder: B,
    ) {
        let id = TypeId::of::<B>();
        let b = IndicatorBuilderAny::wrap(builder);
        self.indicator_builders.insert(id, Box::new(b));
    }

    pub fn register_indicator(
        &mut self,
        symbol: &str,
        interval: Duration,
        key: &str,
    ) -> Result<bool> {
        let _span = warn_span!("indicator", symbol, ?interval, key).entered();
        if self.has_indicator(symbol, interval, key) {
            return Ok(false);
        }

        let mut indicator = None;
        for builder in self.indicator_builders.values() {
            if let Ok(instance) = builder.build(key) {
                indicator.replace(instance);
                break;
            }
        }

        let indicator =
            indicator.ok_or_else(|| key.unexpected("indicator key"))?;

        let deps = indicator.deps();
        for dep in deps {
            self.register_indicator(symbol, interval, dep)?;
        }

        let slots =
            match self.items.iter_mut().find(|item| item.symbol == symbol) {
                Some(i) => i,
                None => self.items.push_mut(HubItem {
                    symbol: symbol.to_owned(),
                    indicators: Default::default(),
                    monitors: Default::default(),
                }),
            };

        slots
            .indicators
            .entry(interval)
            .or_default()
            .push(indicator);

        debug!("added");

        Ok(true)
    }

    pub fn register_monitor_builder<B: Builder<Target: Monitor> + 'static>(
        &mut self,
        builder: B,
    ) {
        let id = TypeId::of::<B>();
        let b = MonitorBuilderAny::wrap(builder);
        self.monitor_builders.insert(id, Box::new(b));
    }

    pub fn register_monitor(
        &mut self,
        symbol: &str,
        interval: Duration,
        key: &str,
    ) -> Result<bool> {
        let _span = warn_span!("monitor", symbol, ?interval, key).entered();

        if self.has_monitor(symbol, interval, key) {
            return Ok(false);
        }

        let mut monitor = None;
        for builder in self.monitor_builders.values() {
            if let Ok(instance) = builder.build(key) {
                monitor.replace(instance);
                break;
            }
        }

        let monitor = monitor.ok_or_else(|| key.unexpected("monitor key"))?;

        let deps = monitor.deps();
        for dep in deps {
            self.register_indicator(symbol, interval, dep)?;
        }

        let slots =
            match self.items.iter_mut().find(|item| item.symbol == symbol) {
                Some(i) => i,
                None => self.items.push_mut(HubItem {
                    symbol: symbol.to_owned(),
                    indicators: Default::default(),
                    monitors: Default::default(),
                }),
            };

        slots.monitors.entry(interval).or_default().push(monitor);

        debug!("added");

        Ok(true)
    }

    pub fn apply_config(&mut self, cfg: Config) -> Result<()> {
        let mut for_all_pairs = None;
        let mut all_pairs = vec![];

        for pair_cfg in cfg.pairs.iter() {
            if pair_cfg.name == "*" {
                for_all_pairs.replace(pair_cfg);
                continue;
            }

            all_pairs.push(&pair_cfg.name);

            self.apply_pair(&pair_cfg.name, pair_cfg)?;
        }

        if let Some(for_all_cfg) = for_all_pairs {
            for pair in all_pairs.iter() {
                self.apply_pair(pair, for_all_cfg)?;
            }
        }

        Ok(())
    }

    fn apply_pair(&mut self, pair: &str, cfg: &Pair) -> Result<()> {
        let mut for_all_intervals = None;
        let mut all_intervals = vec![];
        for interval_cfg in cfg.intervals.iter() {
            let Some(interval) = interval_cfg.name.as_ref().copied() else {
                for_all_intervals.replace(interval_cfg);
                continue;
            };

            all_intervals.push(interval);

            self.apply_interval(pair, interval, interval_cfg)?;
        }

        if let Some(for_all_cfg) = for_all_intervals {
            for interval in all_intervals {
                self.apply_interval(pair, interval, for_all_cfg)?;
            }
        }

        Ok(())
    }

    fn apply_interval(
        &mut self,
        pair: &str,
        interval: Duration,
        cfg: &Interval,
    ) -> Result<()> {
        for m in cfg.monitors.iter() {
            self.register_monitor(pair, interval, m)?;
        }
        Ok(())
    }
}
