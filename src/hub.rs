use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::{indicator, prelude::*};

pub type HubIndicator = Box<dyn Indicator<Output = Box<dyn Any>>>;
pub type HubIndicatorBuilder = Box<dyn Builder<Target = HubIndicator>>;
pub type HubMonitor = Box<dyn Monitor>;

pub struct Hub {
    indicator_builders: HashMap<TypeId, HubIndicatorBuilder>,
    indicators: HashMap<String, HubIndicator>,
    monitors: HashMap<String, HubMonitor>,
}

impl Default for Hub {
    fn default() -> Self {
        let mut hub = Hub {
            indicator_builders: Default::default(),
            indicators: Default::default(),
            monitors: Default::default(),
        };

        hub.register_indicator_builder(indicator::base::BaseExtractorBuilder);
        hub.register_indicator_builder(
            indicator::bollinger::BollingerBandBuilder,
        );
        hub.register_indicator_builder(indicator::macd::MacdBuilder);
        hub.register_indicator_builder(indicator::cross::MaCrossBuilder);
        hub.register_indicator_builder(indicator::distance::DistanceBuilder);
        hub.register_indicator_builder(indicator::position::PositionBuilder);

        hub
    }
}

impl Hub {
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

    pub fn register_indicator(&mut self, key: &str) -> Result<bool> {
        if self.indicators.contains_key(key) {
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
            self.register_indicator(dep)?;
        }

        self.indicators.insert(key.to_owned(), indicator);

        Ok(true)
    }

    pub fn register_monitor(&mut self, monitor: HubMonitor) -> Result<bool> {
        let key = monitor.key();
        if self.monitors.contains_key(key) {
            return Ok(false);
        }

        let deps = monitor.deps();
        for dep in deps {
            self.register_indicator(dep)?;
        }

        self.monitors.insert(key.to_lowercase(), monitor);
        Ok(true)
    }
}
