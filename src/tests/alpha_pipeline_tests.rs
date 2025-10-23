use crate::alpha::{AlphaPipeline, CorrelationAlpha};
use crate::data::HyperliquidData;
use crate::features::{
    compute_feature_set, Feature, LagReturnFeature, RsiFeature, VolatilityFeature,
};
use crate::report::AlphaReport;
use crate::signals::{SignalGenerator, ThresholdSignal};
use crate::strategy::AlphaDrivenStrategy;
use crate::unified_data::OrderSide;
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

    for i in 0..50 {
        let base = 100.0 + (i as f64 * 0.3);
        datetime.push(tz.timestamp_opt(i as i64, 0).unwrap());
        open.push(base);
        high.push(base + 0.8);
        low.push(base - 0.8);
        close.push(base + ((i % 5) as f64 - 2.0) * 0.1);
        volume.push(1_000.0 + i as f64 * 10.0);
        funding_rates.push(0.0001);
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
fn end_to_end_alpha_pipeline_flow() {
    let data = mock_data();
    let feature_set = compute_feature_set(
        &data,
        vec![
            boxed_feature(RsiFeature { period: 6 }),
            boxed_feature(VolatilityFeature { window: 5 }),
            boxed_feature(LagReturnFeature { lag: 2 }),
        ],
    );

    let pipeline = AlphaPipeline::new(&data, feature_set, 1);
    let evaluations = pipeline.evaluate_all(&CorrelationAlpha);
    assert!(!evaluations.is_empty());

    let filtered = evaluations.filter_by_ic(0.01);
    let report: AlphaReport = filtered.to_report();
    assert!(report.len() <= 3);

    if report.is_empty() {
        return;
    }

    let best = report.best_by_ic(1);
    let evaluation = best[0];
    let generator = ThresholdSignal { threshold: 0.5 };
    let signals = generator.generate(evaluation);
    assert_eq!(signals.len(), evaluation.scores.len());

    let strategy = AlphaDrivenStrategy::new("BTC", signals, 1.0);
    let orders = strategy.generate_orders(&data);
    if !orders.is_empty() {
        assert!(orders.iter().all(|order| order.quantity > 0.0));
        assert!(orders
            .iter()
            .all(|order| order.side == OrderSide::Buy || order.side == OrderSide::Sell));
    }
}
fn boxed_feature<F>(feature: F) -> Box<dyn Feature + Send + Sync>
where
    F: Feature + Send + Sync + 'static,
{
    Box::new(feature)
}
