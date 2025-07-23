#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use chrono::{DateTime, FixedOffset, Utc};
    
    use crate::risk_manager::{RiskManager, RiskError, AssetClass, VolatilityData};
    use crate::trading_mode::RiskConfig;
    use crate::trading_mode_impl::{Position, OrderRequest, OrderSide, OrderType, TimeInForce};
    
    // Helper function to create a test position
    fn create_test_position(symbol: &str, size: f64, entry_price: f64, current_price: f64) -> Position {
        Position {
            symbol: symbol.to_string(),
            size,
            entry_price,
            current_price,
            unrealized_pnl: (current_price - entry_price) * size,
            realized_pnl: 0.0,
            funding_pnl: 0.0,
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
        }
    }
    
    // Helper function to create a test order
    fn create_test_order(symbol: &str, side: OrderSide, quantity: f64, price: Option<f64>) -> OrderRequest {
        OrderRequest {
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Limit,
            quantity,
            price,
            reduce_only: false,
            time_in_force: TimeInForce::GoodTillCancel,
        }
    }
    
    #[test]
    fn test_asset_class_mapping() {
        let config = RiskConfig::default();
        let portfolio_value = 10000.0;
        let risk_manager = RiskManager::new(config, portfolio_value);
        
        // Test default asset class mappings
        assert_eq!(risk_manager.get_asset_class("BTC"), Some(&AssetClass::Crypto));
        assert_eq!(risk_manager.get_asset_class("ETH"), Some(&AssetClass::Crypto));
        assert_eq!(risk_manager.get_asset_class("USDT"), Some(&AssetClass::Stablecoin));
        assert_eq!(risk_manager.get_asset_class("UNI"), Some(&AssetClass::Defi));
        assert_eq!(risk_manager.get_asset_class("DOT"), Some(&AssetClass::Layer1));
        assert_eq!(risk_manager.get_asset_class("MATIC"), Some(&AssetClass::Layer2));
        assert_eq!(risk_manager.get_asset_class("DOGE"), Some(&AssetClass::Meme));
        assert_eq!(risk_manager.get_asset_class("APE"), Some(&AssetClass::NFT));
        assert_eq!(risk_manager.get_asset_class("AXS"), Some(&AssetClass::Gaming));
        
        // Test unknown asset
        assert_eq!(risk_manager.get_asset_class("UNKNOWN"), None);
    }
    
    #[test]
    fn test_volatility_based_position_sizing() {
        let mut config = RiskConfig::default();
        config.volatility_sizing_factor = 0.5; // 50% volatility impact
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Create price history with high volatility
        let high_volatility_prices = vec![
            100.0, 110.0, 95.0, 115.0, 90.0, 120.0, 85.0, 125.0, 80.0, 130.0,
            95.0, 115.0, 90.0, 120.0, 85.0, 125.0, 80.0, 130.0, 95.0, 115.0,
            90.0, 120.0, 85.0, 125.0, 80.0, 130.0, 95.0, 115.0, 90.0, 120.0
        ];
        
        // Create price history with low volatility
        let low_volatility_prices = vec![
            100.0, 101.0, 102.0, 101.5, 102.5, 103.0, 102.0, 103.5, 104.0, 103.5,
            104.5, 105.0, 104.5, 105.5, 106.0, 105.5, 106.5, 107.0, 106.5, 107.5,
            108.0, 107.5, 108.5, 109.0, 108.5, 109.5, 110.0, 109.5, 110.5, 111.0
        ];
        
        // Update volatility data
        risk_manager.update_volatility_data("BTC".to_string(), high_volatility_prices);
        risk_manager.update_volatility_data("USDT".to_string(), low_volatility_prices);
        
        // Get volatility data
        let btc_volatility = risk_manager.get_volatility_data("BTC").unwrap();
        let usdt_volatility = risk_manager.get_volatility_data("USDT").unwrap();
        
        // Verify that BTC has higher volatility than USDT
        assert!(btc_volatility.daily_volatility > usdt_volatility.daily_volatility);
        
        // Test position sizing with high volatility asset
        let mut positions = HashMap::new();
        let order_btc = create_test_order("BTC", OrderSide::Buy, 0.1, Some(10000.0));
        
        // With high volatility, the position size should be reduced
        // The max position size is 10% of portfolio = $1000
        // But with high volatility, it should be less than $1000
        if let Err(RiskError::PositionSizeExceeded { message }) = risk_manager.validate_order(&order_btc, &positions) {
            assert!(message.contains("volatility-adjusted limit"));
        } else {
            panic!("Expected PositionSizeExceeded error for high volatility asset");
        }
        
        // Test with smaller position that should be allowed
        let order_btc_small = create_test_order("BTC", OrderSide::Buy, 0.05, Some(10000.0));
        assert!(risk_manager.validate_order(&order_btc_small, &positions).is_ok());
        
        // Test position sizing with low volatility asset
        let order_usdt = create_test_order("USDT", OrderSide::Buy, 900.0, Some(1.0));
        // This should be allowed since USDT has low volatility
        assert!(risk_manager.validate_order(&order_usdt, &positions).is_ok());
    }
    
    #[test]
    fn test_correlation_limits() {
        let mut config = RiskConfig::default();
        config.max_position_correlation = 0.7; // Maximum correlation of 0.7
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Create price histories with high correlation
        let prices1 = vec![
            100.0, 102.0, 104.0, 106.0, 108.0, 110.0, 112.0, 114.0, 116.0, 118.0,
            120.0, 122.0, 124.0, 126.0, 128.0, 130.0, 132.0, 134.0, 136.0, 138.0,
            140.0, 142.0, 144.0, 146.0, 148.0, 150.0, 152.0, 154.0, 156.0, 158.0
        ];
        
        let prices2 = vec![
            200.0, 204.0, 208.0, 212.0, 216.0, 220.0, 224.0, 228.0, 232.0, 236.0,
            240.0, 244.0, 248.0, 252.0, 256.0, 260.0, 264.0, 268.0, 272.0, 276.0,
            280.0, 284.0, 288.0, 292.0, 296.0, 300.0, 304.0, 308.0, 312.0, 316.0
        ];
        
        // Update correlation data
        risk_manager.update_correlation_data("BTC".to_string(), "ETH".to_string(), &prices1, &prices2);
        
        // Verify high correlation
        let correlation = risk_manager.get_correlation("BTC", "ETH").unwrap();
        assert!(correlation > 0.9); // Should be very highly correlated
        
        // Create positions and orders
        let mut positions = HashMap::new();
        positions.insert(
            "BTC".to_string(),
            create_test_position("BTC", 0.05, 10000.0, 10000.0)
        );
        
        // Try to add a highly correlated position in the same direction
        let order_eth = create_test_order("ETH", OrderSide::Buy, 0.5, Some(2000.0));
        
        // This should fail due to high correlation
        if let Err(RiskError::CorrelationLimitExceeded { symbol1, symbol2, correlation, max_correlation }) = risk_manager.validate_order(&order_eth, &positions) {
            assert_eq!(symbol1, "ETH");
            assert_eq!(symbol2, "BTC");
            assert!(correlation > max_correlation);
        } else {
            panic!("Expected CorrelationLimitExceeded error");
        }
        
        // Test with opposite direction (should be allowed despite correlation)
        let order_eth_opposite = create_test_order("ETH", OrderSide::Sell, 0.5, Some(2000.0));
        assert!(risk_manager.validate_order(&order_eth_opposite, &positions).is_ok());
    }
    
    #[test]
    fn test_portfolio_concentration_limits() {
        let mut config = RiskConfig::default();
        config.max_concentration_pct = 0.25; // 25% maximum concentration
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Set asset classes
        risk_manager.set_asset_class("BTC".to_string(), AssetClass::Crypto);
        risk_manager.set_asset_class("ETH".to_string(), AssetClass::Crypto);
        risk_manager.set_asset_class("SOL".to_string(), AssetClass::Crypto);
        
        // Create positions
        let mut positions = HashMap::new();
        positions.insert(
            "BTC".to_string(),
            create_test_position("BTC", 0.1, 10000.0, 10000.0) // $1000 in BTC
        );
        positions.insert(
            "ETH".to_string(),
            create_test_position("ETH", 0.5, 2000.0, 2000.0) // $1000 in ETH
        );
        
        // Total crypto exposure: $2000 (20% of portfolio)
        
        // Try to add more crypto exposure
        let order_sol = create_test_order("SOL", OrderSide::Buy, 50.0, Some(20.0)); // $1000 in SOL
        
        // This should be allowed (total crypto would be 30%, just over the 25% limit)
        if let Err(RiskError::ConcentrationLimitExceeded { asset_class, concentration_pct, max_concentration_pct }) = risk_manager.validate_order(&order_sol, &positions) {
            assert_eq!(asset_class, "Crypto");
            assert!(concentration_pct > max_concentration_pct);
        }
        
        // Try with a smaller order that should be allowed
        let order_sol_small = create_test_order("SOL", OrderSide::Buy, 25.0, Some(20.0)); // $500 in SOL
        // Total crypto would be 25%, which is the limit
        assert!(risk_manager.validate_order(&order_sol_small, &positions).is_ok());
    }
    
    #[test]
    fn test_drawdown_limits() {
        let mut config = RiskConfig::default();
        config.max_drawdown_pct = 0.10; // 10% maximum drawdown
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Update portfolio value with small drawdown
        assert!(risk_manager.update_portfolio_value_with_history(9500.0, -500.0).is_ok()); // 5% drawdown
        
        // Verify drawdown is tracked
        let metrics = risk_manager.get_portfolio_metrics();
        assert!(metrics.max_drawdown > 0.04 && metrics.max_drawdown < 0.06);
        
        // Update with larger drawdown
        assert!(risk_manager.update_portfolio_value_with_history(8900.0, -600.0).is_ok()); // 11% drawdown
        
        // Verify emergency stop is activated
        assert!(risk_manager.should_stop_trading());
        
        // Try to place an order
        let positions = HashMap::new();
        let order = create_test_order("BTC", OrderSide::Buy, 0.1, Some(10000.0));
        assert!(risk_manager.validate_order(&order, &positions).is_err());
    }
    
    #[test]
    fn test_portfolio_volatility_limits() {
        let mut config = RiskConfig::default();
        config.max_portfolio_volatility_pct = 5.0; // 5% maximum portfolio volatility
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Create price history with high volatility
        let high_volatility_prices = vec![
            100.0, 110.0, 95.0, 115.0, 90.0, 120.0, 85.0, 125.0, 80.0, 130.0,
            95.0, 115.0, 90.0, 120.0, 85.0, 125.0, 80.0, 130.0, 95.0, 115.0,
            90.0, 120.0, 85.0, 125.0, 80.0, 130.0, 95.0, 115.0, 90.0, 120.0
        ];
        
        // Update volatility data
        risk_manager.update_volatility_data("BTC".to_string(), high_volatility_prices);
        
        // Get volatility data
        let btc_volatility = risk_manager.get_volatility_data("BTC").unwrap();
        
        // Verify that BTC has high volatility
        assert!(btc_volatility.daily_volatility > config.max_portfolio_volatility_pct);
        
        // Try to add a large position in a highly volatile asset
        let positions = HashMap::new();
        let order_btc = create_test_order("BTC", OrderSide::Buy, 0.2, Some(10000.0)); // $2000 in BTC (20% of portfolio)
        
        // This should fail due to high volatility
        if let Err(RiskError::VolatilityLimitExceeded { current_volatility_pct, max_volatility_pct }) = risk_manager.validate_order(&order_btc, &positions) {
            assert!(current_volatility_pct > max_volatility_pct);
        } else {
            panic!("Expected VolatilityLimitExceeded error");
        }
        
        // Try with a smaller position that should be allowed
        let order_btc_small = create_test_order("BTC", OrderSide::Buy, 0.05, Some(10000.0)); // $500 in BTC (5% of portfolio)
        assert!(risk_manager.validate_order(&order_btc_small, &positions).is_ok());
    }
    
    #[test]
    fn test_emergency_stop_from_drawdown() {
        let mut config = RiskConfig::default();
        config.max_drawdown_pct = 0.10; // 10% maximum drawdown
        
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Update portfolio value with increasing drawdown
        assert!(risk_manager.update_portfolio_value_with_history(9800.0, -200.0).is_ok()); // 2% drawdown
        assert!(risk_manager.update_portfolio_value_with_history(9500.0, -300.0).is_ok()); // 5% drawdown
        assert!(risk_manager.update_portfolio_value_with_history(9000.0, -500.0).is_ok()); // 10% drawdown
        
        // This should trigger emergency stop
        let result = risk_manager.update_portfolio_value_with_history(8900.0, -100.0);
        assert!(result.is_err());
        
        if let Err(RiskError::DrawdownLimitExceeded { current_drawdown_pct, max_drawdown_pct }) = result {
            assert!(current_drawdown_pct > max_drawdown_pct);
        } else {
            panic!("Expected DrawdownLimitExceeded error");
        }
        
        // Verify emergency stop is activated
        assert!(risk_manager.should_stop_trading());
    }
    
    #[test]
    fn test_value_at_risk_calculation() {
        let config = RiskConfig::default();
        let portfolio_value = 10000.0;
        let mut risk_manager = RiskManager::new(config, portfolio_value);
        
        // Add historical portfolio values with some volatility
        for i in 1..=100 {
            let random_change = (i % 7) as f64 - 3.0; // Values between -3 and +3
            let new_value = portfolio_value + random_change * 100.0;
            risk_manager.update_portfolio_value_with_history(new_value, random_change * 100.0).ok();
        }
        
        // Get portfolio metrics
        let metrics = risk_manager.get_portfolio_metrics();
        
        // Verify VaR values are calculated
        assert!(metrics.var_95 > 0.0);
        assert!(metrics.var_99 > 0.0);
        
        // 99% VaR should be higher than 95% VaR
        assert!(metrics.var_99 >= metrics.var_95);
    }
}