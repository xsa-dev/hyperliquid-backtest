# Hyperliquid Backtester

[![Crates.io](https://img.shields.io/crates/v/hyperliquid-backtest.svg)](https://crates.io/crates/hyperliquid-backtest)
[![Documentation](https://docs.rs/hyperliquid-backtest/badge.svg)](https://docs.rs/hyperliquid-backtest)
[![License](https://img.shields.io/crates/l/hyperliquid-backtest.svg)](https://github.com/xsa-dev/hyperliquid-backtest#license)
[![Rust](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)
[![Build Status](https://github.com/xsa-dev/hyperliquid-backtest/workflows/CI/badge.svg)](https://github.com/xsa-dev/hyperliquid-backtest/actions)

A comprehensive Rust library that integrates Hyperliquid trading data with the rs-backtester framework to enable sophisticated backtesting of trading strategies using real Hyperliquid market data, including perpetual futures mechanics and funding rate calculations.

## ✨ Features

- 🚀 **Async Data Fetching**: Efficiently fetch historical OHLC data from Hyperliquid API using the official SDK
- 💰 **Funding Rate Support**: Complete funding rate data and perpetual futures mechanics
- 🔄 **Seamless Integration**: Drop-in replacement for rs-backtester with enhanced features
- 📊 **Enhanced Reporting**: Comprehensive metrics including funding PnL and arbitrage analysis
- ⚡ **High Performance**: Optimized for large datasets and complex multi-asset strategies
- 🛡️ **Type Safety**: Comprehensive error handling with detailed error messages
- 📈 **Advanced Strategies**: Built-in funding arbitrage and enhanced technical indicators
- 🔧 **Developer Friendly**: Extensive documentation, examples, and migration guides
- 📝 **Structured Logging**: Built-in logging and debugging support with configurable output
- 🔴 **Real-Time Monitoring**: Live trading monitoring with alerts and performance tracking
- 📱 **Trading Modes**: Support for backtesting, paper trading, and live trading modes
- 🎯 **Risk Management**: Advanced risk controls and position management
- 📊 **Unified Data Interface**: Consistent API across different trading modes
- 🔔 **Alert System**: Configurable alerts for market conditions and performance metrics

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
hyperliquid-backtest = "0.1.1"
tokio = { version = "1.0", features = ["full"] }
chrono = { version = "0.4", features = ["serde"] }
```

### System Requirements

- **Rust**: 1.70 or later
- **Operating System**: Linux, macOS, or Windows
- **Memory**: Minimum 4GB RAM (8GB+ recommended for large datasets)
- **Network**: Internet connection required for Hyperliquid API access

## 🚀 Quick Start

### Working Data Fetching Example

```rust
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use chrono::{Duration, TimeZone, Utc};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize Hyperliquid client
    let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await?;
    
    // Define time range (last 7 days)
    let now = Utc::now();
    let start_time = (now - Duration::days(7)).timestamp_millis() as u64;
    let end_time = now.timestamp_millis() as u64;
    
    // Fetch BTC/USDC 1-hour candles
    let candles = info_client
        .candles_snapshot("BTC".to_string(), "1h".to_string(), start_time, end_time)
        .await?;
    
    println!("✅ Successfully fetched {} candles!", candles.len());
    
    // Access candle data
    if let Some(first_candle) = candles.first() {
        println!("First candle: Open=${}, Close=${}, Volume={}", 
            first_candle.open, first_candle.close, first_candle.vlm);
    }
    
    Ok(())
}
```

### Working Backtesting Example

```rust
use hyperliquid_backtest::prelude::*;
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use chrono::{Duration, TimeZone, Utc, FixedOffset};

#[tokio::main]
async fn main() -> Result<(), HyperliquidBacktestError> {
    // Initialize logging
    init_logger();
    
    // Fetch data using the working SDK approach
    let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await?;
    
    let end_time = Utc::now();
    let start_time = end_time - Duration::days(7);
    let start_timestamp = start_time.timestamp_millis() as u64;
    let end_timestamp = end_time.timestamp_millis() as u64;
    
    let candles = info_client
        .candles_snapshot("BTC".to_string(), "1h".to_string(), start_timestamp, end_timestamp)
        .await?;
    
    // Convert to internal format
    let mut datetime = Vec::new();
    let mut open = Vec::new();
    let mut high = Vec::new();
    let mut low = Vec::new();
    let mut close = Vec::new();
    let mut volume = Vec::new();
    
    for candle in &candles {
        let timestamp = Utc.timestamp_millis_opt(candle.time_open as i64).unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        datetime.push(timestamp);
        open.push(candle.open.parse::<f64>().unwrap_or(0.0));
        high.push(candle.high.parse::<f64>().unwrap_or(0.0));
        low.push(candle.low.parse::<f64>().unwrap_or(0.0));
        close.push(candle.close.parse::<f64>().unwrap_or(0.0));
        volume.push(candle.vlm.parse::<f64>().unwrap_or(0.0));
    }
    
    let data = HyperliquidData::with_ohlc_data(
        "BTC".to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
    )?;
    
    // Create strategy and run backtest
    let strategy = enhanced_sma_cross(data.to_rs_backtester_data(), 10, 30, Default::default());
    
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    )?;
    
    backtest.calculate_with_funding()?;
    let report = backtest.enhanced_report()?;
    
    println!("📊 Backtest Results:");
    println!("Total Return: {:.2}%", report.total_return * 100.0);
    println!("Trading PnL: ${:.2}", report.trading_pnl);
    println!("Sharpe Ratio: {:.3}", report.sharpe_ratio);
    
    Ok(())
}
```

### Real-Time Monitoring Example

```rust
use hyperliquid_backtest::prelude::*;
use hyperliquid_backtest::real_time_monitoring::{MonitoringServer, MonitoringManager};
use hyperliquid_backtest::live_trading::AlertLevel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize monitoring server
    let port = 8080;
    let mut server = MonitoringServer::new(port);
    server.start().await?;
    
    // Create monitoring manager
    let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
    
    // Add alert handler
    manager.add_alert_handler(|alert| {
        println!("Alert: {} - {}", alert.level, alert.message);
    });
    
    // Send alerts
    manager.send_alert(AlertLevel::Info, "System started", None, None)?;
    manager.send_alert(AlertLevel::Warning, "High volatility detected", Some("BTC"), None)?;
    
    // Update performance metrics
    manager.update_performance_metrics(
        10000.0, // current_balance
        100.0,   // daily_pnl
        500.0,   // total_pnl
        0.6,     // win_rate
        1.5,     // sharpe_ratio
        5.0,     // max_drawdown_pct
        2        // positions_count
    )?;
    
    println!("Monitoring server running on port {}", port);
    
    // Keep server running
    tokio::signal::ctrl_c().await?;
    Ok(())
}
```

## 📚 Usage Guide

### Data Fetching with Working SDK

The library now uses the official Hyperliquid Rust SDK for reliable data fetching:

```rust
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};

// Initialize client
let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await?;

// Fetch data for different intervals
let candles = info_client
    .candles_snapshot("BTC".to_string(), "1h".to_string(), start_time, end_time)
    .await?;

// Supported intervals: "1m", "5m", "15m", "1h", "4h", "1d"
// Supported coins: BTC, ETH, SOL, AVAX, MATIC, ATOM, and many more
```

### Trading Modes

The library supports multiple trading modes through the unified interface:

```rust
use hyperliquid_backtest::prelude::*;

// Backtesting mode
let backtest_mode = TradingMode::Backtest;
let mut backtest = HyperliquidBacktest::new(data, strategy, 10000.0, commission)?;

// Paper trading mode
let paper_mode = TradingMode::PaperTrade;
let mut paper_trader = PaperTradingEngine::new(config)?;

// Live trading mode (with safety controls)
let live_mode = TradingMode::LiveTrade;
let mut live_trader = LiveTradingEngine::new(config)?;
```

### Real-Time Monitoring

Monitor your trading performance in real-time:

```rust
use hyperliquid_backtest::real_time_monitoring::*;

// Start monitoring server
let mut server = MonitoringServer::new(8080);
server.start().await?;

// Create monitoring manager
let mut manager = MonitoringManager::new(TradingMode::LiveTrade);

// Add custom alert handlers
manager.add_alert_handler(|alert| {
    match alert.level {
        AlertLevel::Critical => send_sms_alert(&alert.message),
        AlertLevel::Warning => send_email_alert(&alert.message),
        _ => log_alert(&alert),
    }
});

// Update metrics
manager.update_performance_metrics(
    current_balance,
    daily_pnl,
    total_pnl,
    win_rate,
    sharpe_ratio,
    max_drawdown_pct,
    positions_count
)?;
```

### Risk Management

Advanced risk controls for live trading:

```rust
use hyperliquid_backtest::risk_manager::*;

let risk_config = RiskConfig {
    max_position_size_pct: 0.1,      // Max 10% per position
    max_daily_loss_pct: 0.02,        // Max 2% daily loss
    stop_loss_pct: 0.05,             // 5% stop loss
    take_profit_pct: 0.1,            // 10% take profit
    max_leverage: 3.0,               // Max 3x leverage
    max_positions: 5,                // Max 5 concurrent positions
    max_drawdown_pct: 0.2,           // Max 20% drawdown
    use_trailing_stop: true,
    trailing_stop_distance_pct: Some(0.02),
};

let risk_manager = RiskManager::new(risk_config);
```

### Enhanced Reporting

Generate comprehensive reports with new metrics:

```rust
// Standard enhanced report
let report = backtest.enhanced_report()?;
println!("Sharpe Ratio: {:.3}", report.sharpe_ratio);
println!("Max Drawdown: {:.2}%", report.max_drawdown * 100.0);

// Funding-specific report
let funding_report = backtest.funding_report()?;
println!("Funding Efficiency: {:.2}", funding_report.funding_efficiency);
println!("Funding Volatility: {:.4}", funding_report.funding_volatility);

// Mode-specific reporting
let mode_report = backtest.mode_report()?;
println!("Win Rate: {:.2}%", mode_report.win_rate * 100.0);
println!("Average Trade Duration: {:.1} hours", mode_report.avg_trade_duration_hours);

// Export to CSV with enhanced data
backtest.export_enhanced_csv("backtest_results.csv")?;
```

### Unified Data Interface

Consistent API across all trading modes:

```rust
use hyperliquid_backtest::unified_data::*;

// Create order requests
let market_order = OrderRequest::market("BTC", OrderSide::Buy, 1.0);
let limit_order = OrderRequest::limit("ETH", OrderSide::Sell, 2.0, 3000.0)
    .reduce_only()
    .with_time_in_force(TimeInForce::FillOrKill);

// Market data
let market_data = MarketData::new(
    "BTC",
    50000.0,
    49990.0,
    50010.0,
    100.0,
    Utc::now(),
);

// Position management
let mut position = Position::new("BTC", 1.0, 50000.0, 51000.0, Utc::now());
position.update_price(52000.0);
position.apply_funding_payment(100.0);
```

## 🔄 API Stability

This crate follows [Semantic Versioning (SemVer)](https://semver.org/):

- **Major version** (0.x.y → 1.0.0): Breaking API changes
- **Minor version** (0.1.x → 0.2.0): New features, backward compatible
- **Patch version** (0.1.0 → 0.1.1): Bug fixes, backward compatible

**Current version: 0.1.1** (Pre-1.0 development phase)

### Stability Guarantees

- ✅ **Public API**: All items in the `prelude` module are considered stable within minor versions
- ✅ **Data Structures**: `HyperliquidData`, `HyperliquidBacktest`, and `HyperliquidCommission` are stable
- ✅ **Error Types**: `HyperliquidBacktestError` variants may be added but not removed in minor versions
- ✅ **Strategy Interface**: `HyperliquidStrategy` trait is stable for implementors
- ✅ **Unified Interface**: `OrderRequest`, `MarketData`, `Position` structures are stable

## 📖 Examples

The library includes comprehensive examples in the `examples/` directory:

### Working Examples (Recommended)
- **`working_data_fetch.rs`**: Reliable data fetching using the official SDK
- **`simple_working_backtest.rs`**: Complete working backtest example
- **`basic_backtest.rs`**: Enhanced basic backtesting workflow

### Advanced Features
- **`real_time_monitoring_example.rs`**: Live monitoring with alerts
- **`trading_mode_example.rs`**: Different trading modes demonstration
- **`paper_trading_example.rs`**: Paper trading with risk management
- **`live_trading_safety_example.rs`**: Safe live trading practices
- **`funding_arbitrage_advanced.rs`**: Advanced funding arbitrage strategies
- **`multi_asset_backtest.rs`**: Multi-asset portfolio backtesting
- **`strategy_comparison.rs`**: Compare multiple strategies
- **`performance_comparison.rs`**: Performance analysis tools

### Data and Export
- **`csv_export_example.rs`**: Data export and analysis
- **`enhanced_csv_export_example.rs`**: Advanced CSV export with funding data
- **`unified_data_example.rs`**: Unified data interface usage

Run examples with:

```bash
# Working examples (recommended to start with)
cargo run --example working_data_fetch
cargo run --example simple_working_backtest

# Advanced features
cargo run --example real_time_monitoring_example
cargo run --example trading_mode_example
cargo run --example paper_trading_example
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
- **Real-Time Monitoring**: <1ms latency for alert processing

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
- Use paper trading mode to test strategies before live deployment
- Monitor your positions and set appropriate risk controls

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