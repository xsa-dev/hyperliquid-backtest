# Hyperliquid Backtester

[![Crates.io](https://img.shields.io/crates/v/hyperliquid-backtest.svg)](https://crates.io/crates/hyperliquid-backtest)
[![Documentation](https://docs.rs/hyperliquid-backtest/badge.svg)](https://docs.rs/hyperliquid-backtest)
[![License](https://img.shields.io/crates/l/hyperliquid-backtest.svg)](https://github.com/xsa-dev/hyperliquid-backtest#license)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![Build Status](https://github.com/xsa-dev/hyperliquid-backtest/workflows/CI/badge.svg)](https://github.com/xsa-dev/hyperliquid-backtest/actions)

A comprehensive Rust library that integrates Hyperliquid trading data with the rs-backtester framework to enable sophisticated backtesting of trading strategies using real Hyperliquid market data, including perpetual futures mechanics and funding rate calculations.

## ✨ Features

- 🚀 **Async Data Fetching**: Efficiently fetch historical OHLC data from Hyperliquid API
- 💰 **Funding Rate Support**: Complete funding rate data and perpetual futures mechanics
- 🔄 **Seamless Integration**: Drop-in replacement for rs-backtester with enhanced features
- 📊 **Enhanced Reporting**: Comprehensive metrics including funding PnL and arbitrage analysis
- ⚡ **High Performance**: Optimized for large datasets and complex multi-asset strategies
- 🛡️ **Type Safety**: Comprehensive error handling with detailed error messages
- 📈 **Advanced Strategies**: Built-in funding arbitrage and enhanced technical indicators
- 🔧 **Developer Friendly**: Extensive documentation, examples, and migration guides
- 📝 **Structured Logging**: Built-in logging and debugging support with configurable output

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
hyperliquid-backtest = "0.1"
tokio = { version = "1.0", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
```

### System Requirements

- **Rust**: 1.70 or later
- **Operating System**: Linux, macOS, or Windows
- **Memory**: Minimum 4GB RAM (8GB+ recommended for large datasets)
- **Network**: Internet connection required for Hyperliquid API access

## 🚀 Quick Start

### Basic Backtesting Example

```rust
use hyperliquid_backtest::prelude::*;
use chrono::{DateTime, FixedOffset, Utc};

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    // Initialize logging (optional but recommended)
    init_logger();
    
    // Define time range (last 30 days)
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (30 * 24 * 60 * 60); // 30 days ago
    
    // Fetch historical data for BTC with 1-hour intervals
    let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
    
    // Create a simple moving average crossover strategy
    let strategy = enhanced_sma_cross(10, 20, Default::default())?;
    
    // Set up backtest with $10,000 initial capital
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    )?;
    
    // Run backtest including funding calculations
    backtest.calculate_with_funding()?;
    
    // Generate comprehensive report
    let report = backtest.enhanced_report()?;
    
    println!("📊 Backtest Results:");
    println!("Total Return: {:.2}%", report.total_return * 100.0);
    println!("Trading PnL: ${:.2}", report.trading_pnl);
    println!("Funding PnL: ${:.2}", report.funding_pnl);
    println!("Sharpe Ratio: {:.3}", report.sharpe_ratio);
    
    Ok(())
}
```

### Funding Arbitrage Strategy

```rust
use hyperliquid_backtest::prelude::*;

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    init_logger_with_level("debug");
    
    let end_time = Utc::now().timestamp() as u64;
    let start_time = end_time - (7 * 24 * 60 * 60); // 7 days ago
    
    let data = HyperliquidData::fetch("ETH", "1h", start_time, end_time).await?;
    
    // Create funding arbitrage strategy with 0.01% threshold
    let strategy = funding_arbitrage_strategy(0.0001)?;
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        50000.0, // Higher capital for arbitrage
        HyperliquidCommission::default(),
    )?;
    
    backtest.calculate_with_funding()?;
    
    // Get detailed funding analysis
    let funding_report = backtest.funding_report()?;
    
    println!("💰 Funding Arbitrage Results:");
    println!("Total Funding Received: ${:.2}", funding_report.total_funding_received);
    println!("Total Funding Paid: ${:.2}", funding_report.total_funding_paid);
    println!("Net Funding PnL: ${:.2}", funding_report.net_funding_pnl);
    println!("Average Funding Rate: {:.4}%", funding_report.avg_funding_rate * 100.0);
    
    Ok(())
}
```

## 📚 Usage Guide

### Data Fetching

The library supports fetching historical data for various cryptocurrencies and time intervals:

```rust
use hyperliquid_backtest::prelude::*;

// Supported intervals: "1m", "5m", "15m", "1h", "4h", "1d"
let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;

// Access OHLC data
println!("Number of candles: {}", data.datetime.len());
println!("Latest close price: ${:.2}", data.close.last().unwrap());

// Access funding rate data
if let Some(latest_funding) = data.funding_rates.last() {
    println!("Latest funding rate: {:.4}%", latest_funding * 100.0);
}
```

### Supported Trading Pairs

The library supports all major cryptocurrencies available on Hyperliquid:

- **Major Pairs**: BTC, ETH, SOL, AVAX, DOGE, etc.
- **DeFi Tokens**: UNI, AAVE, COMP, MKR, etc.
- **Layer 1s**: ADA, DOT, ATOM, NEAR, etc.
- **Meme Coins**: SHIB, PEPE, WIF, etc.

### Commission Structure

Configure realistic trading fees based on Hyperliquid's fee structure:

```rust
use hyperliquid_backtest::prelude::*;

// Default Hyperliquid fees
let commission = HyperliquidCommission::default(); // 0.02% maker, 0.05% taker

// Custom fee structure
let custom_commission = HyperliquidCommission {
    maker_rate: 0.0001,  // 0.01% maker fee
    taker_rate: 0.0003,  // 0.03% taker fee
    funding_enabled: true,
};
```

### Strategy Development

Create custom strategies using the built-in framework:

```rust
use hyperliquid_backtest::prelude::*;

// Enhanced SMA crossover with funding awareness
let strategy = enhanced_sma_cross(
    10,  // Short period
    20,  // Long period
    FundingAwareConfig {
        funding_weight: 0.1,
        min_funding_threshold: 0.0001,
    }
)?;

// Funding arbitrage strategy
let arb_strategy = funding_arbitrage_strategy(0.0005)?; // 0.05% threshold
```

### Enhanced Reporting

Generate comprehensive reports with funding-specific metrics:

```rust
// Standard enhanced report
let report = backtest.enhanced_report()?;
println!("Sharpe Ratio: {:.3}", report.sharpe_ratio);
println!("Max Drawdown: {:.2}%", report.max_drawdown * 100.0);

// Funding-specific report
let funding_report = backtest.funding_report()?;
println!("Funding Efficiency: {:.2}", funding_report.funding_efficiency);
println!("Funding Volatility: {:.4}", funding_report.funding_volatility);

// Export to CSV
backtest.export_enhanced_csv("backtest_results.csv")?;
```

### Logging and Debugging

Configure logging for development and production:

```rust
use hyperliquid_backtest::prelude::*;

// Basic logging setup
init_logger(); // INFO level by default

// Debug logging
init_logger_with_level("debug");

// Environment variable control
// RUST_LOG=debug cargo run --example basic_backtest
// HYPERLIQUID_LOG_FORMAT=json cargo run
// HYPERLIQUID_LOG_FILE=backtest.log cargo run
```

## 🔄 API Stability

This crate follows [Semantic Versioning (SemVer)](https://semver.org/):

- **Major version** (0.x.y → 1.0.0): Breaking API changes
- **Minor version** (0.1.x → 0.2.0): New features, backward compatible
- **Patch version** (0.1.0 → 0.1.1): Bug fixes, backward compatible

**Current version: 0.1.0** (Pre-1.0 development phase)

### Stability Guarantees

- ✅ **Public API**: All items in the `prelude` module are considered stable within minor versions
- ✅ **Data Structures**: `HyperliquidData`, `HyperliquidBacktest`, and `HyperliquidCommission` are stable
- ✅ **Error Types**: `HyperliquidBacktestError` variants may be added but not removed in minor versions
- ✅ **Strategy Interface**: `HyperliquidStrategy` trait is stable for implementors

## 📖 Examples

The library includes comprehensive examples in the `examples/` directory:

- **`basic_backtest.rs`**: Simple backtesting workflow
- **`funding_arbitrage_advanced.rs`**: Advanced funding arbitrage strategies
- **`multi_asset_backtest.rs`**: Multi-asset portfolio backtesting
- **`csv_export_example.rs`**: Data export and analysis
- **`performance_comparison.rs`**: Strategy performance comparison
- **`simple_data_fetching.rs`**: Data fetching and exploration

Run examples with:

```bash
cargo run --example basic_backtest
cargo run --example funding_arbitrage_advanced
```

## 🛠️ Advanced Features

### Performance Monitoring

Track performance of operations with built-in spans:

```rust
use hyperliquid_backtest::prelude::*;
use tracing::Instrument;

async fn fetch_and_backtest() -> Result<(), HyperliquidBacktestError> {
    let span = performance_span("full_backtest", &[
        ("symbol", "BTC"),
        ("interval", "1h"),
        ("days", "30")
    ]);
    
    async {
        let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
        // ... rest of backtest logic
        Ok(())
    }
    .instrument(span)
    .await
}
```

### Error Handling

Comprehensive error handling with detailed context:

```rust
use hyperliquid_backtest::prelude::*;

match HyperliquidData::fetch("INVALID", "1h", start, end).await {
    Ok(data) => println!("Success!"),
    Err(HyperliquidBacktestError::HyperliquidApi(msg)) => {
        eprintln!("API Error: {}", msg);
        // Handle API-specific errors
    },
    Err(HyperliquidBacktestError::UnsupportedInterval(interval)) => {
        eprintln!("Unsupported interval: {}", interval);
        eprintln!("Supported intervals: 1m, 5m, 15m, 1h, 4h, 1d");
    },
    Err(e) => eprintln!("Other error: {}", e),
}
```

### Migration from rs-backtester

This library is designed as a drop-in enhancement to rs-backtester:

```rust
// Before (rs-backtester)
use rs_backtester::prelude::*;
let data = Data::from_csv("data.csv")?;

// After (hyperliquid-backtest)
use hyperliquid_backtest::prelude::*;
let data = HyperliquidData::fetch("BTC", "1h", start, end).await?;
let rs_data = data.to_rs_backtester_data(); // Convert if needed
```

## 🧪 Testing

Run the test suite:

```bash
# Run all tests
cargo test

# Run with logging
RUST_LOG=debug cargo test

# Run specific test module
cargo test --test integration_tests

# Run benchmarks
cargo bench
```

## 📊 Performance

The library is optimized for performance with large datasets:

- **Memory Efficient**: Streaming data processing for large time ranges
- **Async Operations**: Non-blocking API calls and data processing
- **Caching**: Intelligent caching of funding rate data
- **Parallel Processing**: Multi-threaded backtesting for complex strategies

Benchmark results on a modern system:
- **Data Fetching**: ~500ms for 30 days of 1h data
- **Backtesting**: ~50ms for 1000 trades with funding calculations
- **Memory Usage**: ~10MB for 30 days of 1h OHLC + funding data

## 🤝 Contributing

Contributions are welcome! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/xsa-dev/hyperliquid-backtest.git
cd hyperliquid-backtest
cargo build
cargo test
```

### Code Style

We use `rustfmt` and `clippy` for code formatting and linting:

```bash
cargo fmt
cargo clippy -- -D warnings
```

## 📄 License

This project is dual-licensed under the MIT OR Apache-2.0 license.

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)

## ⚠️ Disclaimer

**This is an experimental library. Use at your own risk. All actions are performed at your own risk.**

This software is for educational and research purposes only. Trading cryptocurrencies involves substantial risk and may not be suitable for all investors. Past performance does not guarantee future results.

**Important Notes:**
- Always test strategies thoroughly before using real capital
- Be aware of API rate limits when fetching large amounts of data
- Funding rates and market conditions can change rapidly
- Consider transaction costs and slippage in live trading

## 🔗 Links

- [Documentation](https://docs.rs/hyperliquid-backtest)
- [Crates.io](https://crates.io/crates/hyperliquid-backtest)
- [GitHub Repository](https://github.com/xsa-dev/hyperliquid-backtest)
- [Hyperliquid Exchange](https://hyperliquid.xyz)
- [rs-backtester](https://github.com/pmagaz/rs-backtester)

## 📞 Support

- **Issues**: [GitHub Issues](https://github.com/xsa-dev/hyperliquid-backtest/issues)
- **Discussions**: [GitHub Discussions](https://github.com/xsa-dev/hyperliquid-backtest/discussions)
- **Documentation**: [docs.rs](https://docs.rs/hyperliquid-backtest)