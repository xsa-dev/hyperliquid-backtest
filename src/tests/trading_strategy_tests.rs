#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use chrono::{DateTime, FixedOffset, Utc};
    use tempfile::tempdir;
    
    use crate::strategies::trading_strategy::{
        TradingStrategy, StrategyConfig, StrategyState, StrategyParam, BaseTradingStrategy
    };
    use crate::unified_data_impl::{
        MarketData, OrderRequest, OrderResult, Signal, FundingPayment,
        SignalDirection, OrderSide, OrderType, OrderStatus
    };
    
    // Mock implementation of TradingStrategy for testing
    struct MockStrategy {
        config: StrategyConfig,
        state: StrategyState,
        signals: HashMap<String, Signal>,
    }
    
    impl MockStrategy {
        fn new(name: &str) -> Self {
            let config = StrategyConfig::new(
                name,
                "Mock strategy for testing",
                "1.0.0",
            )
            .with_number_param("threshold", 0.01)
            .with_bool_param("use_funding", true)
            .with_string_param("mode", "aggressive");
            
            let state = StrategyState::new(
                name,
                "1.0.0",
                Utc::now().with_timezone(&FixedOffset::east(0)),
            );
            
            Self {
                config,
                state,
                signals: HashMap::new(),
            }
        }
    }
    
    impl TradingStrategy for MockStrategy {
        fn name(&self) -> &str {
            &self.config.name
        }
        
        fn config(&self) -> &StrategyConfig {
            &self.config
        }
        
        fn config_mut(&mut self) -> &mut StrategyConfig {
            &mut self.config
        }
        
        fn update_config(&mut self, config: StrategyConfig) -> Result<(), String> {
            if config.name != self.config.name {
                return Err(format!(
                    "Strategy name mismatch: expected {}, found {}",
                    self.config.name,
                    config.name
                ));
            }
            
            self.config = config;
            Ok(())
        }
        
        fn state(&self) -> &StrategyState {
            &self.state
        }
        
        fn state_mut(&mut self) -> &mut StrategyState {
            &mut self.state
        }
        
        fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String> {
            // Simple mock implementation that generates orders based on price
            let signal = if data.price > data.mid_price() {
                // Price above mid - buy signal
                let signal = Signal {
                    symbol: data.symbol.clone(),
                    direction: SignalDirection::Buy,
                    strength: 0.8,
                    timestamp: data.timestamp,
                    metadata: HashMap::new(),
                };
                
                self.signals.insert(data.symbol.clone(), signal);
                
                vec![OrderRequest::market(&data.symbol, OrderSide::Buy, 1.0)]
            } else {
                // Price below mid - sell signal
                let signal = Signal {
                    symbol: data.symbol.clone(),
                    direction: SignalDirection::Sell,
                    strength: 0.7,
                    timestamp: data.timestamp,
                    metadata: HashMap::new(),
                };
                
                self.signals.insert(data.symbol.clone(), signal);
                
                vec![OrderRequest::market(&data.symbol, OrderSide::Sell, 1.0)]
            };
            
            // Update state with last price
            self.state_mut().set_metric("last_price", data.price);
            self.state_mut().update_timestamp(data.timestamp);
            
            Ok(signal)
        }
        
        fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String> {
            // Update state with fill information
            self.state_mut().set_position(&fill.symbol, match fill.side {
                OrderSide::Buy => fill.filled_quantity,
                OrderSide::Sell => -fill.filled_quantity,
            });
            
            if let Some(price) = fill.average_price {
                self.state_mut().set_metric(&format!("{}_fill_price", fill.symbol), price);
            }
            
            Ok(())
        }
        
        fn on_funding_payment(&mut self, payment: &FundingPayment) -> Result<(), String> {
            // Update state with funding payment
            self.state_mut().set_metric(
                &format!("{}_funding_payment", payment.symbol),
                payment.amount,
            );
            
            Ok(())
        }
        
        fn get_current_signals(&self) -> HashMap<String, Signal> {
            self.signals.clone()
        }
    }
    
    #[test]
    fn test_strategy_config() {
        let mut strategy = MockStrategy::new("TestStrategy");
        
        // Test initial configuration
        assert_eq!(strategy.name(), "TestStrategy");
        assert_eq!(strategy.config().description, "Mock strategy for testing");
        assert_eq!(strategy.config().version, "1.0.0");
        
        // Test parameter access
        assert_eq!(strategy.config().get_number("threshold"), Some(0.01));
        assert_eq!(strategy.config().get_bool("use_funding"), Some(true));
        assert_eq!(strategy.config().get_string("mode"), Some(&"aggressive".to_string()));
        
        // Test parameter update
        strategy.config_mut().parameters.insert(
            "threshold".to_string(),
            StrategyParam::Number(0.02),
        );
        assert_eq!(strategy.config().get_number("threshold"), Some(0.02));
        
        // Test full config update
        let new_config = StrategyConfig::new(
            "TestStrategy",
            "Updated description",
            "1.0.1",
        )
        .with_number_param("threshold", 0.03)
        .with_bool_param("use_funding", false);
        
        strategy.update_config(new_config).unwrap();
        assert_eq!(strategy.config().description, "Updated description");
        assert_eq!(strategy.config().version, "1.0.1");
        assert_eq!(strategy.config().get_number("threshold"), Some(0.03));
        assert_eq!(strategy.config().get_bool("use_funding"), Some(false));
        
        // Test config update with name mismatch
        let invalid_config = StrategyConfig::new(
            "DifferentStrategy",
            "Invalid",
            "1.0.0",
        );
        
        assert!(strategy.update_config(invalid_config).is_err());
    }
    
    #[test]
    fn test_strategy_state() {
        let mut strategy = MockStrategy::new("TestStrategy");
        
        // Test initial state
        assert_eq!(strategy.state().name, "TestStrategy");
        assert_eq!(strategy.state().version, "1.0.0");
        assert!(strategy.state().positions.is_empty());
        
        // Test state updates
        strategy.state_mut().set_position("BTC", 1.5);
        strategy.state_mut().set_metric("profit", 100.0);
        
        assert_eq!(strategy.state().positions.get("BTC"), Some(&1.5));
        assert_eq!(strategy.state().metrics.get("profit"), Some(&100.0));
        
        // Test custom data
        let custom_data = vec![1, 2, 3, 4, 5];
        strategy.state_mut().set_custom_data("test_array", &custom_data).unwrap();
        
        let retrieved: Option<Vec<i32>> = strategy.state().get_custom_data("test_array").unwrap();
        assert_eq!(retrieved, Some(custom_data));
    }
    
    #[test]
    fn test_strategy_persistence() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("strategy_config.json");
        let state_path = temp_dir.path().join("strategy_state.json");
        
        // Create and configure strategy
        let mut strategy = MockStrategy::new("PersistenceTest");
        strategy.config_mut().parameters.insert(
            "threshold".to_string(),
            StrategyParam::Number(0.05),
        );
        strategy.state_mut().set_position("ETH", 2.5);
        strategy.state_mut().set_metric("max_drawdown", 0.15);
        
        // Save config and state
        strategy.save_config(&config_path).unwrap();
        strategy.save_state(&state_path).unwrap();
        
        // Create a new strategy instance
        let mut new_strategy = MockStrategy::new("PersistenceTest");
        
        // Load config and state
        new_strategy.load_config(&config_path).unwrap();
        new_strategy.load_state(&state_path).unwrap();
        
        // Verify loaded data
        assert_eq!(new_strategy.config().get_number("threshold"), Some(0.05));
        assert_eq!(new_strategy.state().positions.get("ETH"), Some(&2.5));
        assert_eq!(new_strategy.state().metrics.get("max_drawdown"), Some(&0.15));
        
        // Test loading with name mismatch
        let mut wrong_strategy = MockStrategy::new("WrongName");
        assert!(wrong_strategy.load_config(&config_path).is_err());
        assert!(wrong_strategy.load_state(&state_path).is_err());
    }
    
    #[test]
    fn test_strategy_lifecycle() {
        let mut strategy = MockStrategy::new("LifecycleTest");
        
        // Create market data
        let market_data = MarketData::new(
            "BTC",
            50000.0,
            49990.0,
            50010.0,
            1000.0,
            Utc::now().with_timezone(&FixedOffset::east(0)),
        );
        
        // Test market data processing
        let orders = strategy.on_market_data(&market_data).unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].symbol, "BTC");
        assert_eq!(orders[0].order_type, OrderType::Market);
        
        // Test signals
        let signals = strategy.get_current_signals();
        assert_eq!(signals.len(), 1);
        assert!(signals.contains_key("BTC"));
        
        // Test order fill processing
        let order_result = OrderResult {
            order_id: "123".to_string(),
            symbol: "BTC".to_string(),
            side: OrderSide::Buy,
            order_type: OrderType::Market,
            requested_quantity: 1.0,
            filled_quantity: 1.0,
            average_price: Some(50000.0),
            status: OrderStatus::Filled,
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
            fees: Some(25.0),
            error: None,
            client_order_id: None,
            metadata: HashMap::new(),
        };
        
        strategy.on_order_fill(&order_result).unwrap();
        assert_eq!(strategy.state().positions.get("BTC"), Some(&1.0));
        
        // Test funding payment processing
        let funding_payment = FundingPayment {
            symbol: "BTC".to_string(),
            rate: 0.0001,
            position_size: 1.0,
            amount: 5.0,
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
        };
        
        strategy.on_funding_payment(&funding_payment).unwrap();
        assert_eq!(
            strategy.state().metrics.get("BTC_funding_payment"),
            Some(&5.0)
        );
    }
    
    #[test]
    fn test_base_trading_strategy() {
        let strategy = BaseTradingStrategy::new(
            "BaseTest",
            "Base strategy implementation",
            "1.0.0",
        );
        
        // Test basic properties
        assert_eq!(strategy.name(), "BaseTest");
        assert_eq!(strategy.config().description, "Base strategy implementation");
        assert_eq!(strategy.config().version, "1.0.0");
        
        // Test default implementations
        let market_data = MarketData::new(
            "BTC",
            50000.0,
            49990.0,
            50010.0,
            1000.0,
            Utc::now().with_timezone(&FixedOffset::east(0)),
        );
        
        let orders = strategy.on_market_data(&market_data).unwrap();
        assert!(orders.is_empty());
        
        let signals = strategy.get_current_signals();
        assert!(signals.is_empty());
    }
}