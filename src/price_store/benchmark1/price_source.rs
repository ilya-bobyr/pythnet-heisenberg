use noise::{NoiseFn, Simplex};
use rand::random;

/// Source of the price information for a given publisher for a given price feed.
pub struct PriceSource {
    pub price_feed_index: u32,
    pub price_mean: i64,
    pub price_range: u64,
    pub confidence_mean: u64,
    pub confidence_range: u64,
    pub noise: Simplex,
}

impl PriceSource {
    pub fn new(
        price_feed_index: u32,
        price_mean: i64,
        price_range: u64,
        confidence_mean: u64,
        confidence_range: u64,
    ) -> Self {
        Self {
            price_feed_index,
            price_mean,
            price_range,
            confidence_mean,
            confidence_range,
            noise: Simplex::new(random()),
        }
    }

    pub fn get(&self, time: f64) -> (i64, u64) {
        let Self {
            price_mean,
            price_range,
            confidence_mean,
            confidence_range,
            noise,
            ..
        } = self;

        let price = {
            // Convert 2D noise into 3D noise to hide the grid.
            let offset = (*price_range as f64) * noise.get([time, time * 0.5]);
            price_mean.saturating_add(offset as i64)
        };

        let confidence = {
            // Convert 2D noise into 3D noise to hide the grid.
            let offset = (*confidence_range as f64) * noise.get([time * 0.5, time]);
            (*confidence_mean as i64)
                .saturating_add(offset as i64)
                .max(0) as u64
        };

        (price, confidence)
    }
}
