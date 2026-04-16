use std::{
    any::{Any, TypeId},
    collections::{BTreeMap, HashMap, HashSet},
};

use tracing::{debug, warn_span};

use crate::{
    config::{ColorTable, Config, Interval, Pair},
    event::K,
    indicator::{
        self,
        base::{CalcKind, ExtractKind},
    },
    monitor::{
        self,
        read::{Read, ReadArgs},
    },
    notifier::Notifier,
    prelude::*,
    util::time::{TBDuration as Duration, compact_format},
};

pub fn line_indent(is_tty: bool) -> &'static str {
    if is_tty { "\t" } else { "    " }
}

pub type HubIndicator = Box<dyn Indicator<Output = Box<dyn Any>>>;
pub type HubIndicatorBuilder = Box<dyn Builder<Target = HubIndicator>>;
pub type HubMonitor = Box<dyn Monitor>;
pub type HubMonitorBuilder = Box<dyn Builder<Target = HubMonitor>>;

pub struct HubItem {
    pub symbol: String,
    pub indicators: BTreeMap<Duration, Vec<HubIndicator>>,
    pub monitors: BTreeMap<Duration, Vec<HubMonitor>>,
    pub reads: BTreeMap<Duration, Read>,
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
        hub.register_indicator_builder(indicator::hl::HlBuilder);

        // monitor builders
        hub.register_monitor_builder(monitor::cross::CrossBuilder);
        hub.register_monitor_builder(monitor::touch::TouchBuilder);
        hub.register_monitor_builder(monitor::hold::HoldBuilder);
        hub.register_monitor_builder(monitor::read::ReadBuilder);
        hub.register_monitor_builder(monitor::rate::RateBuilder);
        hub.register_monitor_builder(monitor::bb::BbBuilder);
        hub.register_monitor_builder(monitor::macd::MacdMonitorBuilder);
        hub.register_monitor_builder(monitor::shadow::ShadowBuilder);
        hub.register_monitor_builder(monitor::pdiff::DiffBuilder);
        hub.register_monitor_builder(monitor::fbq::FBQBuilder);
        hub.register_monitor_builder(monitor::reach::ReachBuilder);

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
                .copied()
                .collect::<HashSet<_>>();

            for d in durations {
                target_set.insert(Target {
                    symbol: item.symbol.clone(),
                    interval: d,
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

        if let Some(item) =
            self.items.iter_mut().find(|item| item.symbol == k.symbol)
        {
            if let Some(monitors) = item.monitors.get_mut(&k.interval) {
                for monitor in monitors {
                    monitor.apply(&kctx);
                }
            }

            if let Some(reads) = item.reads.get_mut(&k.interval) {
                reads.apply(&kctx);
            }
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

    pub fn collect_state_msgs(&self, lines: &mut Vec<String>, is_tty: bool) {
        let mut temp_msgs = Vec::new();
        let mut perm_msgs = Vec::new();

        let states = self.states();

        let indent = line_indent(is_tty);

        for hstate in states {
            for (d, sts) in hstate.states {
                let mut temp_combined: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();
                let mut perm_combined: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();

                for st in sts {
                    if let Some((t, msg)) = st.temp.as_ref() {
                        let msg = if is_tty {
                            msg.tty.clone()
                        } else {
                            msg.normal.clone()
                        };

                        temp_combined.entry(*t).or_default().push(msg);
                    }

                    if let Some((t, msg)) = st.perm.as_ref() {
                        let msg = if is_tty {
                            msg.tty.clone()
                        } else {
                            msg.normal.clone()
                        };

                        perm_combined.entry(*t).or_default().push(msg);
                    }
                }

                for (t, msgs) in temp_combined {
                    temp_msgs.push(format!(
                        "{indent}{}/{d}@{}: {}",
                        hstate.symbol,
                        compact_format(&t, d),
                        msgs.join("  ")
                    ));
                }

                for (t, msgs) in perm_combined {
                    perm_msgs.push(format!(
                        "{indent}{}/{d}@{}: {}",
                        hstate.symbol,
                        compact_format(&t, d),
                        msgs.join("  ")
                    ));
                }
            }
        }

        if !temp_msgs.is_empty() {
            lines.push("TEMP STATES:".to_owned());
            lines.extend(temp_msgs);
        }

        if !perm_msgs.is_empty() {
            lines.push("PERM STATES:".to_owned());
            lines.extend(perm_msgs);
        }
    }

    pub fn collect_read_msgs(&self, lines: &mut Vec<String>, is_tty: bool) {
        let mut read_msgs = Vec::new();
        let indent = line_indent(is_tty);

        for item in self.items.iter() {
            for (d, read) in item.reads.iter() {
                let st = read.state();
                let Some((t, msg)) = st.temp.as_ref() else {
                    continue;
                };

                let content = if is_tty { &msg.tty } else { &msg.normal };

                read_msgs.push(format!(
                    "{indent}{}/{d}@{}:{content}",
                    item.symbol,
                    compact_format(t, *d),
                ));
            }
        }

        if !read_msgs.is_empty() {
            lines.push("READ:".to_owned());
            lines.extend(read_msgs);
        }
    }

    pub fn notify_read_msgs(&self, latest: &BTreeMap<String, Decimal>) {
        let mut read_lines = Vec::new();
        self.collect_read_msgs(&mut read_lines, false);
        if read_lines.is_empty() {
            return;
        }

        let mut latest_line = String::new();
        latest_line.push_str("Latest: ");
        for (n, (s, p)) in latest.iter().enumerate() {
            if n != 0 {
                latest_line.push_str(" | ");
            }

            latest_line.push_str(&format!("{s}@{}", p.round_dp(2)));
        }

        read_lines.push(latest_line);

        for n in self.notifiers.iter() {
            _ = n.process("reads", &read_lines);
        }
    }

    pub fn collect_alert_msgs(
        &mut self,
        lines: &mut Vec<String>,
        is_tty: bool,
        skip: bool,
    ) {
        let mut normal_lines = Vec::new();
        let mut print_lines = Vec::new();
        let indent = line_indent(is_tty);
        for item in self.items.iter_mut() {
            for (d, hubms) in item.monitors.iter_mut() {
                let mut normal_alerts: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();

                let mut tty_alerts: BTreeMap<OffsetDateTime, Vec<String>> =
                    BTreeMap::new();

                for m in hubms.iter_mut() {
                    let alert_msgs = m.take_alerts();
                    for (t, msg) in alert_msgs {
                        normal_alerts.entry(t).or_default().push(msg.normal);
                        tty_alerts.entry(t).or_default().push(msg.tty);
                    }
                }

                let mut normal_formatted_alerts = Vec::new();

                for (t, amsgs) in normal_alerts {
                    normal_formatted_alerts.push(format!(
                        "@{}:{}",
                        compact_format(&t, *d),
                        amsgs.join(" ")
                    ));
                }

                if !normal_formatted_alerts.is_empty() {
                    normal_lines.push(format!(
                        "{indent}{}/{d}: {}",
                        item.symbol,
                        normal_formatted_alerts.join("  ")
                    ));
                }

                if is_tty {
                    let mut tty_formatted_alerts = Vec::new();

                    for (t, amsgs) in tty_alerts {
                        tty_formatted_alerts.push(format!(
                            "@{}:{}",
                            compact_format(&t, *d),
                            amsgs.join(" ")
                        ));
                    }

                    if !tty_formatted_alerts.is_empty() {
                        print_lines.push(format!(
                            "{indent}{}/{d}: {}",
                            item.symbol,
                            tty_formatted_alerts.join("  ")
                        ));
                    }
                }
            }
        }

        if normal_lines.is_empty() || skip {
            return;
        }

        for n in self.notifiers.iter() {
            _ = n.process("alerts", &normal_lines);
        }

        lines.push("\x07ALERTS:".to_owned());
        if print_lines.is_empty() {
            lines.extend(normal_lines);
        } else {
            lines.extend(print_lines);
        }
    }

    pub fn clear_terminated_monitors(&mut self) {
        for item in self.items.iter_mut() {
            for monitors in item.monitors.values_mut() {
                monitors.retain(|m| !m.terminated());
            }
        }
    }

    pub fn remove_once_monitors(&mut self) -> usize {
        let mut removed = 0;
        for item in self.items.iter_mut() {
            for monitors in item.monitors.values_mut() {
                let before = monitors.len();
                monitors.retain(|m| !m.is_once());
                let after = monitors.len();

                removed += before - after;
            }
        }

        removed
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

    fn has_read(&self, symbol: &str, interval: Duration) -> bool {
        let Some(item) = self.items.iter().find(|item| item.symbol == symbol)
        else {
            return false;
        };

        item.reads.contains_key(&interval)
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
        raw_key: &str,
    ) -> Result<bool> {
        let _span = warn_span!("indicator", symbol, ?interval).entered();

        let mut indicator = None;
        for builder in self.indicator_builders.values() {
            if let Ok(instance) = builder.build(raw_key) {
                indicator.replace(instance);
                break;
            }
        }

        let indicator =
            indicator.ok_or_else(|| raw_key.unexpected("indicator key"))?;

        let key = indicator.key();

        if self.has_indicator(symbol, interval, key) {
            return Ok(false);
        }

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
                    reads: Default::default(),
                }),
            };

        debug!(key, "added");
        slots
            .indicators
            .entry(interval)
            .or_default()
            .push(indicator);

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
        raw_key: &str,
        once_only: bool,
    ) -> Result<bool> {
        let _span = warn_span!("monitor", symbol, ?interval).entered();

        let mut monitor = None;
        for builder in self.monitor_builders.values() {
            if let Ok(instance) = builder.build(raw_key) {
                monitor.replace(instance);
                break;
            }
        }

        let monitor =
            monitor.ok_or_else(|| raw_key.unexpected("monitor key"))?;

        let is_once = monitor.is_once();
        if once_only && !is_once {
            return Err(is_once.unexpected("for once_only"));
        }

        let key = monitor.key();

        if self.has_monitor(symbol, interval, key) {
            return Ok(false);
        }

        let deps = monitor.deps();
        for dep in deps {
            self.register_indicator(symbol, interval, dep)?;
        }

        let slots =
            match self.items.iter_mut().find(|item| item.symbol == symbol) {
                Some(i) => {
                    debug!("use exist item");
                    i
                }
                None => {
                    debug!("add new item");
                    self.items.push_mut(HubItem {
                        symbol: symbol.to_owned(),
                        indicators: Default::default(),
                        monitors: Default::default(),
                        reads: Default::default(),
                    })
                }
            };

        debug!(key, "added");
        slots.monitors.entry(interval).or_default().push(monitor);

        Ok(true)
    }

    pub fn register_reads(
        &mut self,
        symbol: &str,
        interval: Duration,
        periods: &[usize],
    ) -> Result<bool> {
        if periods.is_empty() {
            return Ok(false);
        }

        let _span = warn_span!("reads", symbol, ?interval, ?periods).entered();

        if self.has_read(symbol, interval) {
            return Ok(false);
        }

        let read_monitor = ReadArgs::new((
            ExtractKind::PriceClose,
            CalcKind::Ema,
            periods.to_vec(),
        ))
        .build()?;

        let deps = read_monitor.deps();
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
                    reads: Default::default(),
                }),
            };

        slots.reads.insert(interval, read_monitor);

        debug!("added");

        Ok(true)
    }

    pub fn show_monitors(&self) {
        debug!("show monitors");
        for item in self.items.iter() {
            for (d, monitors) in item.monitors.iter() {
                let _span =
                    warn_span!("monitors", symbol = item.symbol, interval = %d)
                        .entered();

                let names =
                    monitors.iter().map(|m| m.key()).collect::<Vec<_>>();
                debug!("added: {names:?}");
            }
        }
    }

    pub fn apply_config(&mut self, cfg: Config) -> Result<()> {
        let mut for_all_pairs = None;
        let mut all_pairs = vec![];

        for pair_cfg in cfg.pairs.iter() {
            if pair_cfg.name == "*" {
                for_all_pairs.replace(pair_cfg);
                continue;
            }

            if !pair_cfg.no_wildcard {
                all_pairs.push(&pair_cfg.name);
            }

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

            if !interval_cfg.no_wildcard {
                all_intervals.push(interval);
            }

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
            self.register_monitor(pair, interval, m, false)?;
        }

        self.register_reads(pair, interval, &cfg.reads)?;
        Ok(())
    }
}
