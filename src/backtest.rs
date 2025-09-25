use chrono::{DateTime, FixedOffset};

/// Minimal representation of a funding payment used in tests and simplified workflows.
#[derive(Debug, Clone, PartialEq)]
pub struct FundingPayment {
    /// Timestamp of the payment.
    pub timestamp: DateTime<FixedOffset>,
    /// Position size in contracts at the time of the payment.
    pub position_size: f64,
    /// Funding rate that was applied for the interval.
    pub funding_rate: f64,
    /// Amount paid or received because of funding. Positive values represent income.
    pub payment_amount: f64,
    /// Mark price when the payment was settled.
    pub mark_price: f64,
}
