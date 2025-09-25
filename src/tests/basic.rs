use chrono::{FixedOffset, Utc};

use crate::unified_data::FundingPayment;
use crate::unified_data::{OrderRequest, OrderSide, OrderType, Position, TimeInForce};

#[test]
fn position_updates_total_pnl_after_price_change_and_funding() {
    let tz = FixedOffset::east_opt(0).expect("valid offset");
    let timestamp = Utc::now().with_timezone(&tz);

    let mut position = Position::new("BTC", 2.0, 100.0, 100.0, timestamp);
    position.update_price(110.0);
    position.apply_funding_payment(1.5);

    let expected_unrealized = 2.0 * (110.0 - 100.0);
    let expected_total = expected_unrealized + 1.5;

    assert!((position.total_pnl() - expected_total).abs() < f64::EPSILON);
}

#[test]
fn limit_order_builder_sets_expected_fields() {
    let order = OrderRequest::limit("ETH", OrderSide::Sell, 1.25, 2000.0);

    assert_eq!(order.symbol, "ETH");
    assert!(matches!(order.side, OrderSide::Sell));
    assert!(matches!(order.order_type, OrderType::Limit));
    assert_eq!(order.quantity, 1.25);
    assert_eq!(order.price, Some(2000.0));
    assert!(!order.reduce_only);
    assert!(matches!(order.time_in_force, TimeInForce::GoodTillCancel));
}

#[test]
fn funding_payment_struct_is_constructible() {
    let tz = FixedOffset::east_opt(0).expect("valid offset");
    let timestamp = Utc::now().with_timezone(&tz);

    let payment = FundingPayment {
        timestamp,
        position_size: 0.75,
        funding_rate: 0.0001,
        payment_amount: 1.2,
        mark_price: 25000.0,
    };

    assert!(payment.payment_amount.is_finite());
    assert_eq!(payment.position_size, 0.75);
}
