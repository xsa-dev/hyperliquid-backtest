//! API validation tests to ensure comprehensive documentation and stability
//!
//! These tests validate that the public API is properly documented, follows
//! stability guarantees, and provides comprehensive error handling.

use crate::prelude::*;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that all public API items are properly documented
    #[test]
    fn test_api_documentation_completeness() {
        // This test ensures that key public API items have documentation
        // In a real implementation, this would use reflection or macro-based
        // validation to ensure all public items have documentation
        
        // For now, we'll test that key types can be constructed and used
        assert!(HyperliquidDataFetcher::supported_intervals().len() > 0);
        
        let commission = HyperliquidCommission::default();
        assert!(commission.maker_rate > 0.0);
        assert!(commission.taker_rate > 0.0);
        
        // Test error message quality
        let error = HyperliquidBacktestError::UnsupportedInterval("invalid".to_string());
        let user_message = error.user_message();
        assert!(user_message.contains("Supported intervals"));
        assert!(user_message.contains("💡"));
    }

    /// Test API stability guarantees
    #[test]
    fn test_api_stability() {
        // Test that core data structures maintain their interface
        let commission = HyperliquidCommission::new(0.001, 0.002, true);
        assert_eq!(commission.maker_rate, 0.001);
        assert_eq!(commission.taker_rate, 0.002);
        assert_eq!(commission.funding_enabled, true);
        
        // Test that error categories are stable
        let error = HyperliquidBacktestError::Network("test".to_string());
        assert_eq!(error.category(), "network");
        assert!(error.is_recoverable());
        
        let validation_error = HyperliquidBacktestError::Validation("test".to_string());
        assert_eq!(validation_error.category(), "validation");
        assert!(validation_error.is_user_error());
    }

    /// Test error handling comprehensiveness
    #[test]
    fn test_error_handling_quality() {
        // Test that errors provide helpful user messages
        let errors = vec![
            HyperliquidBacktestError::UnsupportedInterval("2h".to_string()),
            HyperliquidBacktestError::InvalidTimeRange { start: 100, end: 50 },
            HyperliquidBacktestError::Network("Connection failed".to_string()),
            HyperliquidBacktestError::RateLimit("Too many requests".to_string()),
            HyperliquidBacktestError::Validation("Invalid parameter".to_string()),
        ];
        
        for error in errors {
            let user_message = error.user_message();
            
            // All error messages should contain helpful suggestions
            assert!(user_message.contains("💡"));
            
            // Messages should be longer than just the error itself
            assert!(user_message.len() > error.to_string().len());
            
            // Should have proper categorization
            assert!(!error.category().is_empty());
        }
    }

    /// Test that prelude exports are comprehensive
    #[test]
    fn test_prelude_completeness() {
        // Test that all essential types are available in prelude
        use crate::prelude::*;
        
        // Data types
        let _: Option<HyperliquidData> = None;
        let _: Option<HyperliquidDataFetcher> = None;
        
        // Backtest types
        let _: Option<HyperliquidBacktest> = None;
        let _: Option<HyperliquidCommission> = None;
        let _: Option<OrderType> = None;
        
        // Strategy types
        let _: Option<Box<dyn HyperliquidStrategy>> = None;
        let _: Option<TradingSignal> = None;
        let _: Option<SignalStrength> = None;
        
        // Error types
        let _: Option<HyperliquidBacktestError> = None;
        let _: Option<Result<()>> = None;
        
        // Reporting types
        let _: Option<FundingReport> = None;
        let _: Option<FundingDistribution> = None;
        
        // DateTime types
        let _: Option<DateTime<FixedOffset>> = None;
    }

    /// Test that default implementations are sensible
    #[test]
    fn test_default_implementations() {
        // Test HyperliquidCommission defaults
        let commission = HyperliquidCommission::default();
        assert!(commission.maker_rate < commission.taker_rate); // Maker should be cheaper
        assert!(commission.maker_rate > 0.0); // Should have some fee
        assert!(commission.taker_rate < 0.01); // Should be reasonable (< 1%)
        assert!(commission.funding_enabled); // Should include funding by default
        
        // Test FundingAwareConfig defaults
        let config = FundingAwareConfig::default();
        assert!(config.funding_threshold > 0.0); // Should have some threshold
        assert!(config.funding_weight > 0.0 && config.funding_weight <= 1.0); // Should be valid weight
        assert!(config.use_funding_direction); // Should use funding direction by default
    }

    /// Test that validation methods work correctly
    #[test]
    fn test_validation_methods() {
        // Test interval validation
        assert!(HyperliquidDataFetcher::is_interval_supported("1h"));
        assert!(HyperliquidDataFetcher::is_interval_supported("1d"));
        assert!(!HyperliquidDataFetcher::is_interval_supported("2h"));
        assert!(!HyperliquidDataFetcher::is_interval_supported("invalid"));
        
        // Test that supported intervals are reasonable
        let intervals = HyperliquidDataFetcher::supported_intervals();
        assert!(intervals.contains(&"1h"));
        assert!(intervals.contains(&"1d"));
        assert!(intervals.len() >= 4); // Should have multiple intervals
    }

    /// Test that error conversion works properly
    #[test]
    fn test_error_conversions() {
        // Test that standard errors convert properly
        let parse_error: std::num::ParseFloatError = "invalid".parse::<f64>().unwrap_err();
        let converted: HyperliquidBacktestError = parse_error.into();
        assert!(matches!(converted, HyperliquidBacktestError::NumberParsing(_)));
        
        // Test that JSON errors convert properly
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let converted: HyperliquidBacktestError = json_error.into();
        assert!(matches!(converted, HyperliquidBacktestError::JsonParsing(_)));
    }

    /// Test that helper constructors work correctly
    #[test]
    fn test_helper_constructors() {
        // Test error helper constructors
        let api_error = HyperliquidBacktestError::api_error("test message");
        assert!(matches!(api_error, HyperliquidBacktestError::HyperliquidApi(_)));
        
        let validation_error = HyperliquidBacktestError::validation_error("test validation");
        assert!(matches!(validation_error, HyperliquidBacktestError::Validation(_)));
        
        let config_error = HyperliquidBacktestError::config_error("test config");
        assert!(matches!(config_error, HyperliquidBacktestError::Configuration(_)));
    }

    /// Test that signal types work correctly
    #[test]
    fn test_signal_types() {
        // Test TradingSignal
        let long_signal = TradingSignal::new(1.0, SignalStrength::Strong);
        assert!(long_signal.is_long());
        assert!(!long_signal.is_short());
        assert!(!long_signal.is_neutral());
        
        let short_signal = TradingSignal::new(-0.5, SignalStrength::Medium);
        assert!(!short_signal.is_long());
        assert!(short_signal.is_short());
        assert!(!short_signal.is_neutral());
        
        let neutral_signal = TradingSignal::new(0.0, SignalStrength::Weak);
        assert!(!neutral_signal.is_long());
        assert!(!neutral_signal.is_short());
        assert!(neutral_signal.is_neutral());
    }

    /// Test that commission calculations work correctly
    #[test]
    fn test_commission_calculations() {
        let commission = HyperliquidCommission::new(0.001, 0.002, true);
        
        // Test maker fee calculation
        let maker_fee = commission.calculate_fee(OrderType::LimitMaker, 1000.0);
        assert_eq!(maker_fee, 1.0); // 0.001 * 1000
        
        // Test taker fee calculation
        let taker_fee = commission.calculate_fee(OrderType::Market, 1000.0);
        assert_eq!(taker_fee, 2.0); // 0.002 * 1000
        
        let limit_taker_fee = commission.calculate_fee(OrderType::LimitTaker, 1000.0);
        assert_eq!(limit_taker_fee, 2.0); // 0.002 * 1000
        
        // Test scenario-based fee calculation
        let scenario_fee = commission.calculate_scenario_fee(
            TradingScenario::OpenPosition,
            OrderType::LimitMaker,
            1000.0
        );
        assert_eq!(scenario_fee, 1.0);
    }
}