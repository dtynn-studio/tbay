use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap, HashSet},
};

use humantime::Duration;
use tracing::{debug, warn_span};

use crate::{
    config::{ColorTable, Config, Interval, Pair},
    event::K,
    indicator, monitor,
    notifier::Notifier,
    prelude::*,
    util::time::format_hhmm,
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

pub struct HubState<'s> {
    pub symbol: String,
    pub states: BTreeMap<Duration, Vec<&'s State>>,
}

pub struct Hub {
    indicator_builders: HashMap<TypeId, HubIndicatorBuilder>,
    monitor_builders: HashMap<TypeId, HubMonitorBuilder>,
    items: Vec<HubItem>,
    notifiers: Vec<Notifier>,
    colors: ColorTable,
}

impl Default for Hub {
    fn default() -> Self {
        let mut hub = Hub {
            indicator_builders: Default::default(),
            monitor_builders: Default::default(),
            items: Default::default(),
            notifiers: Default::default(),
            colors: Default::default(),
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
        hub.register_monitor_builder(monitor::read::ReadBuilder);

        hub
    }
}

impl Hub {
    pub fn targets(&self) -> Vec<Target> {
        let mut target_set = HashSet::new();

        for item in self.items.iter() {
            let durations = item
                .indicators
                .keys()
                .chain(item.monitors.keys())
                .collect::<HashSet<_>>();

            for d in durations {
                target_set.insert(Target {
                    symbol: item.symbol.clone(),
                    interval: *d,
                });
            }
        }

        let mut targets = Vec::from_iter(target_set);
        targets.sort_by(|left, right| {
            if left.interval == right.interval {
                left.symbol.cmp(&right.symbol)
            } else {
                left.interval.cmp(&right.interval)
            }
        });
        targets
    }

    pub fn apply_k(&mut self, k: K) {
        let Some(indicators) = self
            .items
            .iter_mut()
            .find(|item| item.symbol == k.symbol)
            .and_then(|item| item.indicators.get_mut(&k.interval))
        else {
            return;
        };

        let mut kctx = KCtx::new(k.raw, self.colors);

        for indicator in indicators {
            if let Some(val) = indicator.apply(&kctx) {
                let key = indicator.key();
                kctx.set_val(key, val);
            }
        }

        let Some(monitors) = self
            .items
            .iter_mut()
            .find(|item| item.symbol == k.symbol)
            .and_then(|item| item.monitors.get_mut(&k.interval))
        else {
            return;
        };

        for monitor in monitors {
            monitor.apply(&kctx);
        }
    }

    pub fn states(&self) -> Vec<HubState<'_>> {
        let mut states = Vec::new();
        for item in self.items.iter() {
            let mut hstate = HubState {
                symbol: item.symbol.clone(),
                states: BTreeMap::new(),
            };

            for (d, ms) in item.monitors.iter() {
                let hs = ms.iter().map(|m| m.state()).collect();
                hstate.states.insert(*d, hs);
            }

            states.push(hstate);
        }

        states
    }

    pub fn collect_state_msgs(&self) -> (Vec<String>, Vec<String>) {
        let mut temp_msgs = Vec::new();
        let mut perm_msgs = Vec::new();

        let states = self.states();

        for hstate in states {
            for (d, sts) in hstate.states {
                let mut temp_combined: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();
                let mut perm_combined: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();

                for st in sts {
                    if let Some((t, msg)) = st.temp.clone() {
                        temp_combined.entry(t).or_default().push(msg);
                    }

                    if let Some((t, msg)) = st.perm.clone() {
                        perm_combined.entry(t).or_default().push(msg);
                    }
                }

                for (t, msgs) in temp_combined {
                    temp_msgs.push(format!(
                        "{}/{d}@{}: {}",
                        hstate.symbol,
                        format_hhmm(&t),
                        msgs.join("  ")
                    ));
                }

                for (t, msgs) in perm_combined {
                    perm_msgs.push(format!(
                        "{}/{d}@{}: {}",
                        hstate.symbol,
                        format_hhmm(&t),
                        msgs.join("  ")
                    ));
                }
            }
        }

        (temp_msgs, perm_msgs)
    }

    pub fn print_state_msgs(&self, temp: bool, perm: bool) -> usize {
        let mut line_count = 0;
        let (temp_msgs, perm_msgs) = self.collect_state_msgs();
        if temp && !temp_msgs.is_empty() {
            line_count += temp_msgs.len() + 2;
            let lines = temp_msgs.join("\n\t");
            println!("TEMP STATES:\n\t{lines}\n");
        }

        if perm && !perm_msgs.is_empty() {
            line_count += perm_msgs.len() + 2;
            let lines = perm_msgs.join("\n\t");
            println!("PERM STATES:\n\t{lines}\n");
        }

        line_count
    }

    pub fn collect_alert_msgs(&mut self) -> Vec<String> {
        let mut lines = Vec::new();
        for item in self.items.iter_mut() {
            for (d, hubms) in item.monitors.iter_mut() {
                let mut alerts: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();
                for m in hubms.iter_mut() {
                    let alert_msgs = m.take_alerts();
                    for (t, msg) in alert_msgs {
                        alerts.entry(t).or_default().push(msg);
                    }
                }

                let mut formatted_alerts = Vec::new();
                for (t, amsgs) in alerts {
                    formatted_alerts.push(format!(
                        "@{}:{}",
                        format_hhmm(&t),
                        amsgs.join(" ")
                    ));
                }

                if !formatted_alerts.is_empty() {
                    lines.push(format!(
                        "{}/{d}: {}",
                        item.symbol,
                        formatted_alerts.join("  ")
                    ));
                }
            }
        }

        lines
    }

    pub fn show_alerts(&mut self) -> usize {
        let alert_lines = self.collect_alert_msgs();
        if alert_lines.is_empty() {
            return 0;
        }

        for n in self.notifiers.iter() {
            _ = n.process("alerts", &alert_lines);
        }

        let line_num = alert_lines.len() + 2;
        let lines = alert_lines.join("\n\t");
        print!("\x07");
        println!("ALERTS:\n\t{lines}\n");

        line_num
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

        for ncfg in cfg.notify {
            let noti = Notifier::new(ncfg)?;
            self.notifiers.push(noti);
        }

        self.colors = cfg.colors;

        Ok(())
    }

    fn apply_pair(&mut self, pair: &str, cfg: &Pair) -> Result<()> {
        let mut for_all_intervals = None;
        let mut all_intervals = vec![];
        for interval_cfg in cfg.intervals.iter() {
            let Some(interval) =
                interval_cfg.name.as_ref().copied().map(Duration::from)
            else {
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
