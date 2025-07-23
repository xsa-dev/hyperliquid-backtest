#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use chrono::{DateTime, FixedOffset, Utc};
    
    use crate::strategies::{
        TradingStrategy, FundingAwareConfig,
        create_funding_arbitrage_strategy, create_enhanced_sma_strategy, create_strategy_template
    };
    use crate::unified_data_impl::{
        MarketData, OrderRequest, OrderResult, FundingPayment,
        OrderSide, OrderType, OrderStatus, SignalDirection
    };
    
    fn create_test_market_data(symbol: &str, price: f64, funding_rate: Option<f64>) -> MarketData {
        let timestamp = Utc::now().with_timezone(&FixedOffset::east(0));
        
        let mut data = MarketData::new(
            symbol,
            price,
            price - 1.0,  // bid
            price + 1.0,  // ask
            1000.0,       // volume
            timestamp,
        );
        
        if let Some(rate) = funding_rate {
            data = data.with_funding_rate(
                rate,
                timestamp + chrono::Duration::hours(8)
            );
        }
        
        data
    }
    
    fn create_test_order_result(symbol: &str, side: OrderSide, quantity: f64, price: f64) -> OrderResult {
        OrderResult {
            order_id: "test_order_123".to_string(),
            symbol: symbol.to_string(),
            side,
            order_type: OrderType::Market,
            requested_quantity: quantity,
            filled_quantity: quantity,
            average_price: Some(price),
            status: OrderStatus::Filled,
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
            fees: Some(price * quantity * 0.0005),  // 0.05% fee
            error: None,
            client_order_id: None,
            metadata: HashMap::new(),
        }
    }
    
    fn create_test_funding_payment(symbol: &str, rate: f64, position_size: f64) -> FundingPayment {
        let amount = position_size * rate;
        
        FundingPayment {
            symbol: symbol.to_string(),
            rate,
            position_size,
            amount,
            timestamp: Utc::now().with_timezone(&FixedOffset::east(0)),
        }
    }
    
    #[test]
    fn test_funding_arbitrage_strategy() {
        // Create strategy with 0.0002 threshold (0.02%)
        let mut strategy = create_funding_arbitrage_strategy(0.0002).unwrap();
        
        // Initialize the strategy
        strategy.initialize().unwrap();
        
        // Test with funding rate below threshold
        let data = create_test_market_data("BTC", 50000.0, Some(0.0001));
        let orders = strategy.on_market_data(&data).unwrap();
        assert!(orders.is_empty(), "Should not generate orders when funding rate is below threshold");
        
        // Test with funding rate above threshold (positive)
        let data = create_test_market_data("BTC", 50000.0, Some(0.0003));
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Buy, "Should be a buy order for positive funding");
        
        // Test with funding rate above threshold (negative)
        let data = create_test_market_data("BTC", 50000.0, Some(-0.0003));
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Sell, "Should be a sell order for negative funding");
        
        // Test order fill handling
        let fill = create_test_order_result("BTC", OrderSide::Buy, 1.0, 50000.0);
        strategy.on_order_fill(&fill).unwrap();
        assert_eq!(strategy.state().positions.get("BTC"), Some(&1.0), "Position should be updated");
        
        // Test funding payment handling
        let payment = create_test_funding_payment("BTC", 0.0003, 1.0);
        strategy.on_funding_payment(&payment).unwrap();
        assert!(strategy.state().metrics.contains_key("BTC_funding_payment"), "Funding payment should be tracked");
        
        // Test signals
        let signals = strategy.get_current_signals();
        assert_eq!(signals.len(), 1, "Should have one signal");
        assert_eq!(signals.get("BTC").unwrap().direction, SignalDirection::Sell, "Last signal should be sell");
        
        // Shutdown the strategy
        strategy.shutdown().unwrap();
    }
    
    #[test]
    fn test_enhanced_sma_strategy() {
        // Create strategy with fast_period=5, slow_period=10
        let mut strategy = create_enhanced_sma_strategy(5, 10, None).unwrap();
        
        // Initialize the strategy
        strategy.initialize().unwrap();
        
        // Feed price data to build history
        for i in 0..15 {
            let price = 50000.0 + (i as f64 * 100.0);
            let data = create_test_market_data("ETH", price, Some(0.0001));
            let _ = strategy.on_market_data(&data).unwrap();
        }
        
        // Now prices are rising, so fast SMA > slow SMA
        let data = create_test_market_data("ETH", 51500.0, Some(0.0001));
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Buy, "Should be a buy order when fast SMA > slow SMA");
        
        // Now feed decreasing prices
        for i in 0..15 {
            let price = 51500.0 - (i as f64 * 100.0);
            let data = create_test_market_data("ETH", price, Some(-0.0001));
            let _ = strategy.on_market_data(&data).unwrap();
        }
        
        // Now prices are falling, so fast SMA < slow SMA
        let data = create_test_market_data("ETH", 50000.0, Some(-0.0001));
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Sell, "Should be a sell order when fast SMA < slow SMA");
        
        // Test with strong negative funding rate (should reinforce sell signal)
        let data = create_test_market_data("ETH", 50000.0, Some(-0.0005));
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Sell, "Should be a sell order with strong negative funding");
        
        // Test order fill handling
        let fill = create_test_order_result("ETH", OrderSide::Sell, 1.0, 50000.0);
        strategy.on_order_fill(&fill).unwrap();
        assert_eq!(strategy.state().positions.get("ETH"), Some(&-1.0), "Position should be updated");
        
        // Test funding payment handling
        let payment = create_test_funding_payment("ETH", -0.0003, -1.0);
        strategy.on_funding_payment(&payment).unwrap();
        assert!(strategy.state().metrics.contains_key("ETH_funding_payment"), "Funding payment should be tracked");
        
        // Test signals
        let signals = strategy.get_current_signals();
        assert_eq!(signals.len(), 1, "Should have one signal");
        assert_eq!(signals.get("ETH").unwrap().direction, SignalDirection::Sell, "Last signal should be sell");
        
        // Shutdown the strategy
        strategy.shutdown().unwrap();
    }
    
    #[test]
    fn test_strategy_template() {
        // Create strategy template
        let mut strategy = create_strategy_template(1.0, true).unwrap();
        
        // Initialize the strategy
        strategy.initialize().unwrap();
        
        // Test with price above mid price
        let data = create_test_market_data("SOL", 100.0, None);
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Buy, "Should be a buy order when price > mid");
        
        // Test with price below mid price
        let mut data = create_test_market_data("SOL", 98.0, None);
        data.bid = 97.0;
        data.ask = 99.0;
        let orders = strategy.on_market_data(&data).unwrap();
        assert_eq!(orders.len(), 1, "Should generate one order");
        assert_eq!(orders[0].side, OrderSide::Sell, "Should be a sell order when price < mid");
        
        // Test order fill handling
        let fill = create_test_order_result("SOL", OrderSide::Sell, 1.0, 98.0);
        strategy.on_order_fill(&fill).unwrap();
        assert_eq!(strategy.state().positions.get("SOL"), Some(&-1.0), "Position should be updated");
        
        // Test funding payment handling
        let payment = create_test_funding_payment("SOL", -0.0002, -1.0);
        strategy.on_funding_payment(&payment).unwrap();
        assert!(strategy.state().metrics.contains_key("SOL_funding_payment"), "Funding payment should be tracked");
        
        // Test signals
        let signals = strategy.get_current_signals();
        assert_eq!(signals.len(), 1, "Should have one signal");
        assert_eq!(signals.get("SOL").unwrap().direction, SignalDirection::Sell, "Last signal should be sell");
        
        // Shutdown the strategy
        strategy.shutdown().unwrap();
    }
    
    #[test]
    fn test_mode_compatibility() {
        // This test verifies that strategies work across different trading modes
        
        // Create strategies
        let mut funding_strategy = create_funding_arbitrage_strategy(0.0002).unwrap();
        let mut sma_strategy = create_enhanced_sma_strategy(5, 10, None).unwrap();
        let mut template_strategy = create_strategy_template(1.0, true).unwrap();
        
        // Initialize strategies
        funding_strategy.initialize().unwrap();
        sma_strategy.initialize().unwrap();
        template_strategy.initialize().unwrap();
        
        // Create market data for different modes
        let backtest_data = create_test_market_data("BTC", 50000.0, Some(0.0003));
        let paper_data = create_test_market_data("ETH", 3000.0, Some(-0.0003));
        let live_data = create_test_market_data("SOL", 100.0, Some(0.0001));
        
        // Test strategies with different data
        let funding_orders = funding_strategy.on_market_data(&backtest_data).unwrap();
        let sma_orders = sma_strategy.on_market_data(&paper_data).unwrap();
        let template_orders = template_strategy.on_market_data(&live_data).unwrap();
        
        // Verify orders were generated
        assert!(!funding_orders.is_empty(), "Funding strategy should generate orders");
        assert!(sma_orders.is_empty(), "SMA strategy should not generate orders yet (not enough history)");
        assert!(!template_orders.is_empty(), "Template strategy should generate orders");
        
        // Create order fills
        let funding_fill = create_test_order_result("BTC", OrderSide::Buy, 1.0, 50000.0);
        let template_fill = create_test_order_result("SOL", OrderSide::Buy, 1.0, 100.0);
        
        // Process order fills
        funding_strategy.on_order_fill(&funding_fill).unwrap();
        template_strategy.on_order_fill(&template_fill).unwrap();
        
        // Verify positions were updated
        assert_eq!(funding_strategy.state().positions.get("BTC"), Some(&1.0), "Funding strategy position should be updated");
        assert_eq!(template_strategy.state().positions.get("SOL"), Some(&1.0), "Template strategy position should be updated");
        
        // Create funding payments
        let funding_payment = create_test_funding_payment("BTC", 0.0003, 1.0);
        let sma_payment = create_test_funding_payment("ETH", -0.0003, -1.0);
        let template_payment = create_test_funding_payment("SOL", 0.0001, 1.0);
        
        // Process funding payments
        funding_strategy.on_funding_payment(&funding_payment).unwrap();
        sma_strategy.on_funding_payment(&sma_payment).unwrap();
        template_strategy.on_funding_payment(&template_payment).unwrap();
        
        // Verify funding payments were tracked
        assert!(funding_strategy.state().metrics.contains_key("BTC_funding_payment"), "Funding payment should be tracked");
        assert!(sma_strategy.state().metrics.contains_key("ETH_funding_payment"), "Funding payment should be tracked");
        assert!(template_strategy.state().metrics.contains_key("SOL_funding_payment"), "Funding payment should be tracked");
        
        // Shutdown strategies
        funding_strategy.shutdown().unwrap();
        sma_strategy.shutdown().unwrap();
        template_strategy.shutdown().unwrap();
    }
}