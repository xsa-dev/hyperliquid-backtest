use crate::trading_mode::{
    TradingMode, TradingModeManager, TradingConfig, RiskConfig, SlippageConfig, ApiConfig
};

#[test]
fn test_trading_mode_display() {
    assert_eq!(TradingMode::Backtest.to_string(), "Backtest");
    assert_eq!(TradingMode::PaperTrade.to_string(), "Paper Trading");
    assert_eq!(TradingMode::LiveTrade.to_string(), "Live Trading");
}

#[test]
fn test_trading_config_creation() {
    let config = TradingConfig::new(10000.0)
        .with_risk_config(RiskConfig::default())
        .with_slippage_config(SlippageConfig::default())
        .with_parameter("test_key", "test_value");
    
    assert_eq!(config.initial_balance, 10000.0);
    assert!(config.risk_config.is_some());
    assert!(config.slippage_config.is_some());
    assert_eq!(config.parameters.get("test_key").unwrap(), "test_value");
}

#[test]
fn test_trading_config_validation() {
    // Valid backtest config
    let backtest_config = TradingConfig::new(10000.0);
    assert!(backtest_config.validate_for_mode(TradingMode::Backtest).is_ok());
    
    // Invalid backtest config (negative balance)
    let invalid_backtest_config = TradingConfig::new(-1000.0);
    assert!(invalid_backtest_config.validate_for_mode(TradingMode::Backtest).is_err());
    
    // Valid paper trading config
    let paper_config = TradingConfig::new(10000.0)
        .with_slippage_config(SlippageConfig::default());
    assert!(paper_config.validate_for_mode(TradingMode::PaperTrade).is_ok());
    
    // Valid live trading config
    let live_config = TradingConfig::new(10000.0)
        .with_risk_config(RiskConfig::default())
        .with_api_config(ApiConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            endpoint: "https://api.example.com".to_string(),
            use_testnet: true,
            timeout_ms: 5000,
        });
    assert!(live_config.validate_for_mode(TradingMode::LiveTrade).is_ok());
    
    // Invalid live trading config (missing API config)
    let invalid_live_config = TradingConfig::new(10000.0)
        .with_risk_config(RiskConfig::default());
    assert!(invalid_live_config.validate_for_mode(TradingMode::LiveTrade).is_err());
}

#[test]
fn test_trading_mode_manager_creation() {
    let config = TradingConfig::new(10000.0);
    let manager = TradingModeManager::new(TradingMode::Backtest, config);
    
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    assert_eq!(manager.config().initial_balance, 10000.0);
}

#[test]
fn test_trading_mode_switching() {
    let config = TradingConfig::new(10000.0)
        .with_slippage_config(SlippageConfig::default())
        .with_risk_config(RiskConfig::default())
        .with_api_config(ApiConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            endpoint: "https://api.example.com".to_string(),
            use_testnet: true,
            timeout_ms: 5000,
        });
    
    let mut manager = TradingModeManager::new(TradingMode::Backtest, config);
    
    // Test valid mode transitions
    assert!(manager.switch_mode(TradingMode::PaperTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::PaperTrade);
    
    assert!(manager.switch_mode(TradingMode::LiveTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::LiveTrade);
    
    assert!(manager.switch_mode(TradingMode::PaperTrade).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::PaperTrade);
    
    assert!(manager.switch_mode(TradingMode::Backtest).is_ok());
    assert_eq!(manager.current_mode(), TradingMode::Backtest);
    
    // Test invalid mode transition (backtest to live)
    let result = manager.switch_mode(TradingMode::LiveTrade);
    assert!(result.is_err());
    if let Err(crate::trading_mode::TradingModeError::UnsupportedModeTransition { from, to }) = result {
        assert_eq!(from, TradingMode::Backtest);
        assert_eq!(to, TradingMode::LiveTrade);
    } else {
        panic!("Expected UnsupportedModeTransition error");
    }
}

#[test]
fn test_config_update() {
    let initial_config = TradingConfig::new(10000.0);
    let mut manager = TradingModeManager::new(TradingMode::Backtest, initial_config);
    
    let new_config = TradingConfig::new(20000.0)
        .with_parameter("test_key", "test_value");
    
    assert!(manager.update_config(new_config).is_ok());
    assert_eq!(manager.config().initial_balance, 20000.0);
    assert_eq!(manager.config().parameters.get("test_key").unwrap(), "test_value");
}