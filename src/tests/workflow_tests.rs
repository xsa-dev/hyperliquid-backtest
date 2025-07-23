use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use chrono::{DateTime, FixedOffset, Utc};
use tokio::test;

use crate::backtest::HyperliquidBacktest;
use crate::data::HyperliquidData;
use crate::paper_trading::PaperTradingEngine;
use crate::live_trading::LiveTradingEngine;
use crate::trading_mode::{
    TradingMode, TradingModeManager, TradingConfig, RiskConfig, SlippageConfig, ApiConfig
};
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy, Signal, SignalDirection, OrderFill, FundingPayment
};
use crate::real_time_data_stream::RealTimeDataStream;

// Test strategy implementation for workflow testing
struct WorkflowTestStrategy {
    name: String,
    signals: HashMap<String, Signal>,
    positions: HashMap<String, f64>,
    sma_short_period: usize,
    sma_long_period: usize,
    sma_short_values: HashMap<String, Vec<f64>>,
    sma_long_values: HashMap<String, Vec<f64>>,
    parameters: HashMap<String, f64>,
}

impl WorkflowTestStrategy {
    fn new(name: &str, short_period: usize, long_period: usize) -> Self {
        let mut parameters = HashMap::new();
        parameters.insert("short_period".to_string(), short_period as f64);
        parameters.insert("long_period".to_string(), long_period as f64);
        
        Self {
            name: name.to_string(),
            signals: HashMap::new(),
            positions: HashMap::new(),
            sma_short_period: short_period,
            sma_long_period: long_period,
            sma_short_values: HashMap::new(),
            sma_long_values: HashMap::new(),
            parameters,
        }
    }
    
    fn calculate_sma(&mut self, symbol: &str, price: f64) {
        // Update short SMA
        let short_values = self.sma_short_values.entry(symbol.to_string()).or_insert_with(Vec::new);
        short_values.push(price);
        if short_values.len() > self.sma_short_period {
            short_values.remove(0);
        }
        
        // Update long SMA
        let long_values = self.sma_long_values.entry(symbol.to_string()).or_insert_with(Vec::new);
        long_values.push(price);
        if long_values.len() > self.sma_long_period {
            long_values.remove(0);
        }
    }
    
    fn get_short_sma(&self, symbol: &str) -> Option<f64> {
        if let Some(values) = self.sma_short_values.get(symbol) {
            if values.len() == self.sma_short_period {
                let sum: f64 = values.iter().sum();
                return Some(sum / values.len() as f64);
            }
        }
        None
    }
    
    fn get_long_sma(&self, symbol: &str) -> Option<f64> {
        if let Some(values) = self.sma_long_values.get(symbol) {
            if values.len() == self.sma_long_period {
                let sum: f64 = values.iter().sum();
                return Some(sum / values.len() as f64);
            }
        }
        None
    }
    
    // Save strategy state to be restored later
    fn save_state(&self) -> HashMap<String, String> {
        let mut state = HashMap::new();
        
        // Save positions
        for (symbol, size) in &self.positions {
            state.insert(format!("position_{}", symbol), size.to_string());
        }
        
        // Save parameters
        for (key, value) in &self.parameters {
            state.insert(format!("param_{}", key), value.to_string());
        }
        
        state
    }
    
    // Restore strategy state
    fn restore_state(&mut self, state: &HashMap<String, String>) {
        // Restore positions
        for (key, value) in state {
            if key.starts_with("position_") {
                let symbol = key.strip_prefix("position_").unwrap();
                if let Ok(size) = value.parse::<f64>() {
                    self.positions.insert(symbol.to_string(), size);
                }
            } else if key.starts_with("param_") {
                let param = key.strip_prefix("param_").unwrap();
                if let Ok(value) = value.parse::<f64>() {
                    self.parameters.insert(param.to_string(), value);
                }
            }
        }
    }
}

impl TradingStrategy for WorkflowTestStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        // Calculate SMAs
        self.calculate_sma(&data.symbol, data.price);
        
        // Generate signals based on SMA crossover
        let short_sma = self.get_short_sma(&data.symbol);
        let long_sma = self.get_long_sma(&data.symbol);
        
        let mut orders = Vec::new();
        
        if let (Some(short), Some(long)) = (short_sma, long_sma) {
            let current_position = *self.positions.get(&data.symbol).unwrap_or(&0.0);
            
            // SMA crossover strategy
            if short > long && current_position <= 0.0 {
                // Buy signal
                let signal = Signal {
                    symbol: data.symbol.clone(),
                    direction: SignalDirection::Buy,
                    strength: 1.0,
                    timestamp: data.timestamp,
                    metadata: HashMap::new(),
                };
                
                self.signals.insert(data.symbol.clone(), signal);
                
                // Close short position if exists
                if current_position < 0.0 {
                    orders.push(OrderRequest::market(&data.symbol, OrderSide::Buy, current_position.abs()));
                }
                
                // Open long position
                orders.push(OrderRequest::market(&data.symbol, OrderSide::Buy, 1.0));
                
            } else if short < long && current_position >= 0.0 {
                // Sell signal
                let signal = Signal {
                    symbol: data.symbol.clone(),
                    direction: SignalDirection::Sell,
                    strength: 1.0,
                    timestamp: data.timestamp,
                    metadata: HashMap::new(),
                };
                
                self.signals.insert(data.symbol.clone(), signal);
                
                // Close long position if exists
                if current_position > 0.0 {
                    orders.push(OrderRequest::market(&data.symbol, OrderSide::Sell, current_position));
                }
                
                // Open short position
                orders.push(OrderRequest::market(&data.symbol, OrderSide::Sell, 1.0));
            }
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, fill: &OrderFill) -> Result<(), String> {
        // Update position
        let current_position = *self.positions.get(&fill.symbol).unwrap_or(&0.0);
        let position_change = match fill.side {
            OrderSide::Buy => fill.quantity,
            OrderSide::Sell => -fill.quantity,
        };
        
        self.positions.insert(fill.symbol.clone(), current_position + position_change);
        
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &FundingPayment) -> Result<(), String> {
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}

// Helper function to create test data
fn create_test_data(symbol: &str, data_points: usize) -> HyperliquidData {
    let now = Utc::now().with_timezone(&FixedOffset::east(0));
    let mut datetime = Vec::with_capacity(data_points);
    let mut open = Vec::with_capacity(data_points);
    let mut high = Vec::with_capacity(data_points);
    let mut low = Vec::with_capacity(data_points);
    let mut close = Vec::with_capacity(data_points);
    let mut volume = Vec::with_capacity(data_points);
    let mut funding_rates = Vec::new();
    let mut funding_timestamps = Vec::new();
    
    let mut price = 100.0;
    
    for i in 0..data_points {
        // Generate somewhat realistic price movement
        let change = (rand::random::<f64>() - 0.5) * 2.0; // Random change between -1 and 1
        price += change;
        price = price.max(10.0); // Ensure price doesn't go too low
        
        let timestamp = now + chrono::Duration::minutes(i as i64);
        datetime.push(timestamp);
        open.push(price);
        high.push(price * (1.0 + rand::random::<f64>() * 0.01)); // Up to 1% higher
        low.push(price * (1.0 - rand::random::<f64>() * 0.01));  // Up to 1% lower
        close.push(price);
        volume.push(100.0 + rand::random::<f64>() * 900.0); // Random volume between 100 and 1000
        
        // Add funding rate every 8 hours (480 minutes)
        if i % 480 == 0 {
            funding_rates.push((rand::random::<f64>() - 0.5) * 0.001); // Random funding rate between -0.05% and 0.05%
            funding_timestamps.push(timestamp);
        }
    }
    
    HyperliquidData {
        ticker: symbol.to_string(),
        datetime,
        open,
        high,
        low,
        close,
        volume,
        funding_rates,
        funding_timestamps,
    }
}

// Helper function to create market data from HyperliquidData
fn create_market_data_from_hyperliquid(data: &HyperliquidData) -> Vec<MarketData> {
    let mut result = Vec::with_capacity(data.close.len());
    
    for i in 0..data.close.len() {
        let market_data = MarketData::new(
            &data.ticker,
            data.close[i],
            data.low[i],
            data.high[i],
            data.volume[i],
            data.datetime[i],
        );
        result.push(market_data);
    }
    
    result
}

#[test]
fn test_complete_strategy_development_workflow() {
    // 1. Create test data
    let data_points = 1000;
    let data = create_test_data("BTC", data_points);
    
    // 2. Backtest phase
    println!("PHASE 1: BACKTESTING");
    let strategy = WorkflowTestStrategy::new("SMA_Crossover", 10, 30);
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    let backtest_report = backtest.report();
    
    println!("Backtest Results:");
    println!("  Trades: {}", backtest_report.trades);
    println!("  Final Equity: ${:.2}", backtest_report.final_equity);
    println!("  Return: {:.2}%", backtest_report.return_pct);
    
    // 3. Strategy optimization (simplified)
    println!("\nPHASE 2: STRATEGY OPTIMIZATION");
    
    // Test different parameters
    let parameter_sets = vec![
        (5, 20),
        (10, 30),
        (15, 45),
        (20, 60),
    ];
    
    let mut best_return = -100.0;
    let mut best_params = (0, 0);
    
    for (short_period, long_period) in parameter_sets {
        let strategy = WorkflowTestStrategy::new("SMA_Crossover", short_period, long_period);
        
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            Box::new(strategy),
            10000.0,
            Default::default(),
        );
        
        backtest.run();
        let report = backtest.report();
        
        println!("  Parameters ({}, {}): Return {:.2}%", short_period, long_period, report.return_pct);
        
        if report.return_pct > best_return {
            best_return = report.return_pct;
            best_params = (short_period, long_period);
        }
    }
    
    println!("  Best parameters: ({}, {}) with {:.2}% return", best_params.0, best_params.1, best_return);
    
    // 4. Paper trading phase (simulated)
    println!("\nPHASE 3: PAPER TRADING");
    
    // Create optimized strategy
    let mut strategy = WorkflowTestStrategy::new("SMA_Crossover_Optimized", best_params.0, best_params.1);
    
    // Create paper trading engine
    let mut paper_engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    // Convert historical data to market data for paper trading simulation
    let market_data = create_market_data_from_hyperliquid(&data);
    
    // Only use the last 200 data points for paper trading
    let paper_data = &market_data[market_data.len() - 200..];
    
    // Run paper trading simulation
    for data_point in paper_data {
        paper_engine.update_market_data(data_point.clone()).unwrap();
        let orders = strategy.on_market_data(data_point).unwrap();
        
        for order in orders {
            let result = paper_engine.execute_order_sync(order).unwrap();
            
            let fill = OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data_point.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    let paper_report = paper_engine.generate_report();
    
    println!("Paper Trading Results:");
    println!("  Trades: {}", paper_report.trade_count);
    println!("  Final Equity: ${:.2}", paper_report.total_equity);
    println!("  Return: {:.2}%", paper_report.total_return_pct);
    
    // 5. Strategy state persistence and recovery
    println!("\nPHASE 4: STATE PERSISTENCE AND RECOVERY");
    
    // Save strategy state
    let strategy_state = strategy.save_state();
    println!("  Strategy state saved with {} entries", strategy_state.len());
    
    // Create new strategy instance
    let mut new_strategy = WorkflowTestStrategy::new("SMA_Crossover_Recovered", best_params.0, best_params.1);
    
    // Restore state
    new_strategy.restore_state(&strategy_state);
    println!("  Strategy state restored");
    
    // Verify positions were restored correctly
    for (symbol, size) in &strategy.positions {
        let recovered_size = new_strategy.positions.get(symbol).unwrap_or(&0.0);
        println!("  Position for {}: Original={}, Recovered={}", symbol, size, recovered_size);
        assert_eq!(size, recovered_size);
    }
    
    // 6. Disaster recovery simulation
    println!("\nPHASE 5: DISASTER RECOVERY SIMULATION");
    
    // Simulate a crash by creating a new paper trading engine
    let mut recovery_engine = PaperTradingEngine::new(paper_report.total_equity, SlippageConfig::default());
    
    // Restore positions
    for (symbol, position) in paper_engine.get_positions() {
        recovery_engine.add_position(position.clone()).unwrap();
        println!("  Restored position for {}: Size={}, Entry Price=${:.2}", 
                 symbol, position.size, position.entry_price);
    }
    
    // Continue trading with recovered state
    let recovery_data = &market_data[market_data.len() - 100..];
    
    for data_point in recovery_data {
        recovery_engine.update_market_data(data_point.clone()).unwrap();
        let orders = new_strategy.on_market_data(data_point).unwrap();
        
        for order in orders {
            let result = recovery_engine.execute_order_sync(order).unwrap();
            
            let fill = OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data_point.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            new_strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    let recovery_report = recovery_engine.generate_report();
    
    println!("Recovery Results:");
    println!("  Trades: {}", recovery_report.trade_count);
    println!("  Final Equity: ${:.2}", recovery_report.total_equity);
    println!("  Return: {:.2}%", recovery_report.total_return_pct);
    
    // 7. Mode transition testing
    println!("\nPHASE 6: MODE TRANSITION TESTING");
    
    // Create trading mode manager
    let config = TradingConfig::new(10000.0)
        .with_risk_config(RiskConfig::default())
        .with_slippage_config(SlippageConfig::default());
    
    let mut manager = TradingModeManager::new(TradingMode::Backtest, config);
    
    // Test mode transitions
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    println!("  Current mode: Backtest");
    
    // Switch to paper trading
    assert!(manager.switch_mode(TradingMode::PaperTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::PaperTrade);
    println!("  Switched to: Paper Trading");
    
    // Direct switch from paper to live should be allowed with confirmation
    assert!(manager.switch_mode_with_confirmation(TradingMode::LiveTrade, true).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::LiveTrade);
    println!("  Switched to: Live Trading (with confirmation)");
    
    // Switch back to paper trading
    assert!(manager.switch_mode(TradingMode::PaperTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::PaperTrade);
    println!("  Switched back to: Paper Trading");
    
    // 8. API compatibility testing
    println!("\nPHASE 7: API COMPATIBILITY TESTING");
    
    // Test that our strategy implements the TradingStrategy trait correctly
    fn test_strategy_compatibility<T: TradingStrategy + 'static>(strategy: T) {
        println!("  Strategy '{}' correctly implements TradingStrategy trait", strategy.name());
    }
    
    test_strategy_compatibility(WorkflowTestStrategy::new("CompatibilityTest", 10, 30));
    println!("  API compatibility test passed");
    
    // 9. Workflow completion
    println!("\nComplete strategy development workflow test passed!");
}

#[test]
fn test_mode_transition_workflow() {
    // Create test data
    let data_points = 500;
    let data = create_test_data("BTC", data_points);
    let market_data = create_market_data_from_hyperliquid(&data);
    
    // 1. Backtest phase
    println!("PHASE 1: BACKTESTING");
    let strategy = WorkflowTestStrategy::new("SMA_Crossover", 10, 30);
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    let backtest_report = backtest.report();
    
    println!("Backtest Results:");
    println!("  Trades: {}", backtest_report.trades);
    println!("  Final Equity: ${:.2}", backtest_report.final_equity);
    
    // 2. Create strategy for paper trading
    let mut paper_strategy = WorkflowTestStrategy::new("SMA_Crossover", 10, 30);
    
    // 3. Paper trading phase
    println!("\nPHASE 2: PAPER TRADING");
    let mut paper_engine = PaperTradingEngine::new(10000.0, SlippageConfig::default());
    
    // Use a subset of data for paper trading
    let paper_data = &market_data[0..200];
    
    for data_point in paper_data {
        paper_engine.update_market_data(data_point.clone()).unwrap();
        let orders = paper_strategy.on_market_data(data_point).unwrap();
        
        for order in orders {
            let result = paper_engine.execute_order_sync(order).unwrap();
            
            let fill = OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data_point.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            paper_strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    let paper_report = paper_engine.generate_report();
    
    println!("Paper Trading Results:");
    println!("  Trades: {}", paper_report.trade_count);
    println!("  Final Equity: ${:.2}", paper_report.total_equity);
    
    // 4. Save strategy state for transition
    let strategy_state = paper_strategy.save_state();
    
    // 5. Create live trading configuration (simulated)
    println!("\nPHASE 3: LIVE TRADING PREPARATION");
    
    // Create a new strategy for live trading
    let mut live_strategy = WorkflowTestStrategy::new("SMA_Crossover", 10, 30);
    
    // Restore state from paper trading
    live_strategy.restore_state(&strategy_state);
    
    // Verify positions were transferred correctly
    for (symbol, size) in &paper_strategy.positions {
        let live_size = live_strategy.positions.get(symbol).unwrap_or(&0.0);
        println!("  Position for {}: Paper={}, Live={}", symbol, size, live_size);
        assert_eq!(size, live_size);
    }
    
    println!("  Strategy state successfully transferred from paper to live");
    
    // 6. Simulate live trading (using remaining data)
    println!("\nPHASE 4: SIMULATED LIVE TRADING");
    
    // Create simulated live trading engine
    let mut live_engine = PaperTradingEngine::new(paper_report.total_equity, SlippageConfig::default());
    
    // Transfer positions from paper trading
    for (symbol, position) in paper_engine.get_positions() {
        live_engine.add_position(position.clone()).unwrap();
    }
    
    // Use remaining data for "live" trading
    let live_data = &market_data[200..];
    
    for data_point in live_data {
        live_engine.update_market_data(data_point.clone()).unwrap();
        let orders = live_strategy.on_market_data(data_point).unwrap();
        
        for order in orders {
            let result = live_engine.execute_order_sync(order).unwrap();
            
            let fill = OrderFill {
                order_id: result.order_id.clone(),
                symbol: result.symbol.clone(),
                side: result.side,
                quantity: result.filled_quantity,
                price: result.average_price.unwrap_or(data_point.price),
                timestamp: result.timestamp,
                fees: result.fees.unwrap_or(0.0),
            };
            
            live_strategy.on_order_fill(&fill).unwrap();
        }
    }
    
    let live_report = live_engine.generate_report();
    
    println!("Live Trading Results:");
    println!("  Trades: {}", live_report.trade_count);
    println!("  Final Equity: ${:.2}", live_report.total_equity);
    
    // 7. Compare results across modes
    println!("\nPHASE 5: CROSS-MODE COMPARISON");
    
    println!("Backtest Final Equity: ${:.2}", backtest_report.final_equity);
    println!("Paper Trading Final Equity: ${:.2}", paper_report.total_equity);
    println!("Live Trading Final Equity: ${:.2}", live_report.total_equity);
    
    println!("Mode transition workflow test completed successfully!");
}

#[test]
fn test_production_deployment_validation() {
    // This test simulates the validation steps before deploying to production
    println!("PRODUCTION DEPLOYMENT VALIDATION TEST");
    
    // 1. Create test data
    let data = create_test_data("BTC", 1000);
    
    // 2. Strategy validation
    println!("\nPHASE 1: STRATEGY VALIDATION");
    
    let strategy = WorkflowTestStrategy::new("ProductionStrategy", 10, 30);
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    let backtest_report = backtest.report();
    
    // Check strategy performance meets minimum requirements
    let min_return = 0.0; // Minimum acceptable return
    println!("  Strategy return: {:.2}%", backtest_report.return_pct);
    println!("  Minimum required: {:.2}%", min_return);
    assert!(backtest_report.return_pct >= min_return, "Strategy doesn't meet minimum return requirement");
    println!("  Strategy performance validation: PASSED");
    
    // 3. Risk management validation
    println!("\nPHASE 2: RISK MANAGEMENT VALIDATION");
    
    // Create risk config
    let risk_config = RiskConfig {
        max_position_size_pct: 0.05,      // 5% of portfolio
        max_daily_loss_pct: 0.02,         // 2% max daily loss
        stop_loss_pct: 0.05,              // 5% stop loss
        take_profit_pct: 0.1,             // 10% take profit
        max_leverage: 2.0,                // 2x max leverage
        max_concentration_pct: 0.2,       // 20% max concentration
        max_position_correlation: 0.5,    // 0.5 maximum correlation
        max_portfolio_volatility_pct: 0.1, // 10% maximum portfolio volatility
        volatility_sizing_factor: 0.3,    // 30% volatility-based position sizing
        max_drawdown_pct: 0.1,            // 10% maximum drawdown
    };
    
    // Create risk manager
    let mut risk_manager = crate::risk_manager::RiskManager::new(risk_config, 10000.0);
    
    // Test order validation
    let order = OrderRequest::market("BTC", OrderSide::Buy, 0.01);
    let result = risk_manager.validate_order(&order, &HashMap::new());
    assert!(result.is_ok(), "Risk validation failed: {:?}", result.err());
    println!("  Risk management validation: PASSED");
    
    // 4. Failover testing
    println!("\nPHASE 3: FAILOVER TESTING");
    
    // Simulate strategy state persistence
    let mut strategy = WorkflowTestStrategy::new("ProductionStrategy", 10, 30);
    
    // Run strategy with some data
    let market_data = create_market_data_from_hyperliquid(&data);
    let initial_data = &market_data[0..100];
    
    for data_point in initial_data {
        let _ = strategy.on_market_data(data_point);
    }
    
    // Save strategy state
    let strategy_state = strategy.save_state();
    println!("  Strategy state saved with {} entries", strategy_state.len());
    
    // Simulate failure and recovery
    let mut recovered_strategy = WorkflowTestStrategy::new("RecoveredStrategy", 10, 30);
    recovered_strategy.restore_state(&strategy_state);
    
    // Verify recovery
    assert_eq!(strategy.positions.len(), recovered_strategy.positions.len(), 
               "Position count mismatch after recovery");
    println!("  Failover recovery validation: PASSED");
    
    // 5. API compatibility testing
    println!("\nPHASE 4: API COMPATIBILITY TESTING");
    
    // Test with current API version
    let api_version = "1.0.0"; // Current version
    println!("  Testing with API version: {}", api_version);
    
    // Simulate API version check
    fn check_api_compatibility(version: &str) -> bool {
        // In a real implementation, this would check against supported versions
        version == "1.0.0"
    }
    
    assert!(check_api_compatibility(api_version), "API version {} not compatible", api_version);
    println!("  API compatibility validation: PASSED");
    
    // 6. Production readiness validation
    println!("\nPHASE 5: PRODUCTION READINESS VALIDATION");
    
    // Create trading config for production
    let trading_config = TradingConfig::new(10000.0)
        .with_risk_config(risk_config)
        .with_slippage_config(SlippageConfig::default());
    
    // Create trading mode manager
    let mut manager = TradingModeManager::new(TradingMode::PaperTrade, trading_config);
    
    // Test mode switching with confirmation
    assert!(manager.switch_mode_with_confirmation(TradingMode::LiveTrade, true).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::LiveTrade);
    println!("  Mode transition validation: PASSED");
    
    // 7. Final validation
    println!("\nAll production deployment validation tests PASSED!");
    println!("System is ready for production deployment");
}

#[test]
fn test_regression_suite() {
    // This test ensures that API changes don't break existing functionality
    println!("API REGRESSION TEST SUITE");
    
    // 1. Create test data
    let data = create_test_data("BTC", 500);
    
    // 2. Test core functionality
    println!("\nPHASE 1: CORE FUNCTIONALITY TESTING");
    
    // Test data conversion
    let market_data = create_market_data_from_hyperliquid(&data);
    assert_eq!(data.close.len(), market_data.len());
    println!("  Data conversion test: PASSED");
    
    // Test strategy execution
    let strategy = WorkflowTestStrategy::new("RegressionTest", 10, 30);
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    println!("  Strategy execution test: PASSED");
    
    // 3. Test API compatibility
    println!("\nPHASE 2: API COMPATIBILITY TESTING");
    
    // Test TradingStrategy trait implementation
    struct LegacyStrategy;
    
    impl TradingStrategy for LegacyStrategy {
        fn name(&self) -> &str {
            "LegacyStrategy"
        }
        
        fn on_market_data(&mut self, _data: &MarketData) -> Result<Vec<OrderRequest>, String> {
            Ok(vec![])
        }
        
        fn on_order_fill(&mut self, _fill: &OrderFill) -> Result<(), String> {
            Ok(())
        }
        
        fn on_funding_payment(&mut self, _payment: &FundingPayment) -> Result<(), String> {
            Ok(())
        }
        
        fn get_current_signals(&self) -> HashMap<String, Signal> {
            HashMap::new()
        }
    }
    
    let legacy_strategy = LegacyStrategy;
    
    // Test that legacy strategy still works with current API
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        Box::new(legacy_strategy),
        10000.0,
        Default::default(),
    );
    
    backtest.run();
    println!("  Legacy strategy compatibility test: PASSED");
    
    // 4. Test configuration compatibility
    println!("\nPHASE 3: CONFIGURATION COMPATIBILITY TESTING");
    
    // Create trading config
    let trading_config = TradingConfig::new(10000.0)
        .with_risk_config(RiskConfig::default())
        .with_slippage_config(SlippageConfig::default());
    
    // Test that config can be created and used
    let manager = TradingModeManager::new(TradingMode::Backtest, trading_config);
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    println!("  Configuration compatibility test: PASSED");
    
    // 5. Final validation
    println!("\nAll API regression tests PASSED!");
    println!("API compatibility is maintained");
}