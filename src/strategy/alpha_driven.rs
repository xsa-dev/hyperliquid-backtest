use crate::data::HyperliquidData;
use crate::signals::SignalValue;
use crate::unified_data::{OrderRequest, OrderSide};

/// Strategy that turns a stream of signals into market orders with a fixed position size.
#[derive(Debug, Clone)]
pub struct AlphaDrivenStrategy {
    symbol: String,
    signals: Vec<SignalValue>,
    quantity: f64,
}

impl AlphaDrivenStrategy {
    /// Create a new strategy instance.
    pub fn new(symbol: impl Into<String>, signals: Vec<SignalValue>, quantity: f64) -> Self {
        Self {
            symbol: symbol.into(),
            signals,
            quantity: quantity.abs(),
        }
    }

    /// Borrow the signals associated with the strategy.
    pub fn signals(&self) -> &[SignalValue] {
        &self.signals
    }

    /// Generate market orders required to follow the signal stream.
    ///
    /// The helper compares consecutive signals and emits the minimal set of
    /// market orders needed to reach the desired exposure. Transitions between
    /// long and short positions are handled by issuing a double-sized order to
    /// close the previous exposure before establishing the new one.
    pub fn generate_orders(&self, data: &HyperliquidData) -> Vec<OrderRequest> {
        if self.signals.is_empty() {
            return Vec::new();
        }

        let mut orders = Vec::new();
        let mut previous = SignalValue::Flat;
        let limit = self.signals.len().min(data.close.len());

        for idx in 0..limit {
            let current = self.signals[idx];
            if let Some((side, quantity)) = transition(previous, current, self.quantity) {
                orders.push(OrderRequest::market(&self.symbol, side, quantity));
            }
            previous = current;
        }

        orders
    }
}

fn transition(
    previous: SignalValue,
    current: SignalValue,
    base_quantity: f64,
) -> Option<(OrderSide, f64)> {
    match (previous, current) {
        (SignalValue::Flat, SignalValue::Long) => Some((OrderSide::Buy, base_quantity)),
        (SignalValue::Flat, SignalValue::Short) => Some((OrderSide::Sell, base_quantity)),
        (SignalValue::Long, SignalValue::Flat) => Some((OrderSide::Sell, base_quantity)),
        (SignalValue::Short, SignalValue::Flat) => Some((OrderSide::Buy, base_quantity)),
        (SignalValue::Long, SignalValue::Short) => Some((OrderSide::Sell, base_quantity * 2.0)),
        (SignalValue::Short, SignalValue::Long) => Some((OrderSide::Buy, base_quantity * 2.0)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signals::SignalValue;
    use chrono::{FixedOffset, TimeZone};

    fn mock_data() -> HyperliquidData {
        let tz = FixedOffset::east_opt(0).unwrap();
        let mut datetime = Vec::new();
        let mut open = Vec::new();
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();
        let mut volume = Vec::new();
        let mut funding_rates = Vec::new();
        for i in 0..6 {
            let price = 100.0 + i as f64;
            datetime.push(tz.timestamp_opt(i as i64, 0).unwrap());
            open.push(price);
            high.push(price + 1.0);
            low.push(price - 1.0);
            close.push(price);
            volume.push(1000.0);
            funding_rates.push(0.0);
        }

        HyperliquidData {
            symbol: "BTC".to_string(),
            datetime,
            open,
            high,
            low,
            close,
            volume,
            funding_rates,
        }
    }

    #[test]
    fn strategy_generates_orders_from_signals() {
        let data = mock_data();
        let signals = vec![
            SignalValue::Flat,
            SignalValue::Long,
            SignalValue::Long,
            SignalValue::Short,
            SignalValue::Flat,
            SignalValue::Short,
        ];
        let strategy = AlphaDrivenStrategy::new("BTC", signals, 1.0);
        let orders = strategy.generate_orders(&data);
        assert_eq!(orders.len(), 4);
        assert_eq!(orders[0].side, OrderSide::Buy);
        assert_eq!(orders[1].side, OrderSide::Sell);
        assert_eq!(orders[2].side, OrderSide::Buy);
        assert_eq!(orders[3].side, OrderSide::Sell);
    }
}
