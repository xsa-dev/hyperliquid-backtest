//! Strategy helpers that bridge evaluated alphas and order generation.
//!
//! The provided [`AlphaDrivenStrategy`] converts a sequence of
//! [`SignalValue`](crate::signals::SignalValue) values into Hyperliquid order
//! requests that can be fed into the existing backtesting infrastructure.

mod alpha_driven;

pub use alpha_driven::AlphaDrivenStrategy;
