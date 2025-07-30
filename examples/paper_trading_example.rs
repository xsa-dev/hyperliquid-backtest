use std::sync::{Arc, Mutex};
use std::time::Duration;
use chrono::{DateTime, FixedOffset, Utc};
use std::collections::HashMap;

use hyperliquid_backtest::prelude::*;

// A simple moving average crossover strategy
struct SimpleSmaStrategy {
    name: String,
    short_period: usize,
    long_period: usize,
    short_values: Vec<f64>,
    long_values: Vec<f64>,
    signals: HashMap<String, Signal>,
    last_prices: HashMap<String, f64>,
}

impl SimpleSmaStrategy {
    fn new(short_period: usize, long_period: usize) -> Self {
        Self {
            name: format!("SMA {}/{} Crossover", short_period, long_period),
            short_period,
            long_period,
            short_values: Vec::new(),
            long_values: Vec::new(),
            signals: HashMap::new(),
            last_prices: HashMap::new(),
        }
    }
    
    fn calculate_sma(&self, values: &[f64], period: usize) -> Option<f64> {
        if values.len() < period {
            return None;
        }
        
        let sum: f64 = values.iter().rev().take(period).sum();
        Some(sum / period as f64)
    }
}

impl TradingStrategy for SimpleSmaStrategy {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        let symbol = &data.symbol;
        let price = data.price;
        
        // Store the price
        self.last_prices.insert(symbol.clone(), price);
        
        // Update price history
        self.short_values.push(price);
        
        // Keep only necessary history
        let max_period = self.short_period.max(self.long_period);
        if self.short_values.len() > max_period * 2 {
            self.short_values.remove(0);
        }
        
        // Calculate SMAs
        let short_sma = self.calculate_sma(&self.short_values, self.short_period);
        let long_sma = self.calculate_sma(&self.short_values, self.long_period);
        
        // Generate signals
        let mut orders = Vec::new();
        
        if let (Some(short), Some(long)) = (short_sma, long_sma) {
            let now = Utc::now().with_timezone(&FixedOffset::east(0));
            
            // Crossover logic
            let signal_direction = if short > long {
                // Short SMA above long SMA - bullish
                SignalDirection::Buy
            } else if short < long {
                // Short SMA below long SMA - bearish
                SignalDirection::Sell
            } else {
                SignalDirection::Neutral
            };
            
            // Create signal
            let signal = Signal {
                symbol: symbol.clone(),
                direction: signal_direction,
                strength: 1.0,
                timestamp: now,
                metadata: HashMap::new(),
            };
            
            // Check if signal changed
            let previous_signal = self.signals.get(symbol);
            let signal_changed = match previous_signal {
                Some(prev) => prev.direction != signal.direction,
                None => true,
            };
            
            // Store the new signal
            self.signals.insert(symbol.clone(), signal.clone());
            
            // Generate orders on signal change
            if signal_changed {
                match signal.direction {
                    SignalDirection::Buy => {
                        // Close any existing short position
                        orders.push(OrderRequest::market(symbol, OrderSide::Buy, 1.0));
                    },
                    SignalDirection::Sell => {
                        // Close any existing long position
                        orders.push(OrderRequest::market(symbol, OrderSide::Sell, 1.0));
                    },
                    _ => {}
                }
            }
        }
        
        Ok(orders)
    }
    
    fn on_order_fill(&mut self, _fill: &OrderResult) -> Result<(), String> {
        // Nothing to do here for this simple strategy
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &FundingPayment) -> Result<(), String> {
        // Nothing to do here for this simple strategy
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        self.signals.clone()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_logger();
    
    println!("Hyperliquid Paper Trading Example");
    println!("================================");
    
    // Create slippage configuration
    let slippage_config = SlippageConfig {
        base_slippage_pct: 0.0005,   // 0.05% base slippage
        volume_impact_factor: 0.1,   // Volume impact factor
        volatility_impact_factor: 0.2, // Volatility impact factor
        random_slippage_max_pct: 0.001, // 0.1% max random component
        simulated_latency_ms: 500,   // 500ms simulated latency
        use_order_book: false,
        max_slippage_pct: 0.002,     // 0.2% max slippage
    };
    
    // Create paper trading engine with $10,000 initial balance
    let mut paper_engine = PaperTradingEngine::new(10000.0, slippage_config);
    
    // Create real-time data stream
    let data_stream = RealTimeDataStream::new().await?;
    let data_stream = Arc::new(Mutex::new(data_stream));
    
    // Set the real-time data stream in the paper trading engine
    paper_engine.set_real_time_data(data_stream.clone());
    
    // Subscribe to market data for BTC
    {
        let mut stream = data_stream.lock().unwrap();
        stream.connect().await?;
        stream.subscribe_ticker("BTC").await?;
        stream.subscribe_order_book("BTC").await?;
        stream.subscribe_funding_rate("BTC").await?;
    }
    
    // Create a simple SMA crossover strategy
    let strategy = SimpleSmaStrategy::new(10, 20);
    let strategy_box: Box<dyn TradingStrategy> = Box::new(strategy);
    
    // Start paper trading in a separate task
    let paper_engine_clone = Arc::new(Mutex::new(paper_engine));
    let paper_engine_for_task = paper_engine_clone.clone();
    
    let task_handle = tokio::spawn(async move {
        let mut engine = paper_engine_for_task.lock().unwrap();
        if let Err(e) = engine.start_simulation(strategy_box).await {
            eprintln!("Error in paper trading simulation: {}", e);
        }
    });
    
    // Let the simulation run for a while
    println!("Paper trading simulation started. Running for 30 seconds...");
    tokio::time::sleep(Duration::from_secs(30)).await;
    
    // Stop the simulation
    {
        let mut engine = paper_engine_clone.lock().unwrap();
        engine.stop_simulation();
    }
    
    // Wait for the task to complete
    let _ = task_handle.await;
    
    // Get the final results
    let report = {
        let engine = paper_engine_clone.lock().unwrap();
        engine.generate_report()
    };
    
    // Print the report
    println!("\nPaper Trading Results:");
    println!("{}", report);
    
    // Print positions
    let positions = {
        let engine = paper_engine_clone.lock().unwrap();
        engine.get_positions().clone()
    };
    
    println!("\nFinal Positions:");
    for (symbol, position) in positions {
        println!("{}: {} @ ${} (PnL: ${:.2})", 
            symbol, 
            position.size, 
            position.current_price,
            position.total_pnl()
        );
    }
    
    // Print trade history
    let trade_log = {
        let engine = paper_engine_clone.lock().unwrap();
        engine.get_trade_log().clone()
    };
    
    println!("\nTrade History:");
    for (i, trade) in trade_log.iter().enumerate() {
        println!("{}. {} {} {} @ ${} (Fees: ${:.2})", 
            i + 1,
            trade.timestamp.format("%Y-%m-%d %H:%M:%S"),
            trade.side,
            trade.quantity,
            trade.price,
            trade.fees
        );
    }
    
    println!("\nExample completed successfully!");
    
    Ok(())
}