//! Enhanced tests for the CSV export functionality

use crate::csv_export::*;
use crate::backtest::{HyperliquidBacktest, HyperliquidCommission, FundingPayment};
use crate::data::HyperliquidData;
use crate::errors::Result;
use crate::tests::mock_data::{
    generate_mock_data, generate_mock_funding_payments,
    generate_position_sequence
};
use chrono::{DateTime, FixedOffset, TimeZone};
use std::path::Path;
use std::fs;
use std::io::Read;

/// Test that EnhancedCsvExport trait is implemented correctly
#[test]
fn test_enhanced_csv_export_trait() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array with constant long position
    let positions = vec![1.0; data.len()];
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Create a temporary file path for testing
    let temp_file = "test_enhanced_export.csv";
    
    // Export to CSV
    backtest.export_to_csv(temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains expected headers
    assert!(content.contains("timestamp"));
    assert!(content.contains("price"));
    assert!(content.contains("position"));
    assert!(content.contains("equity"));
    assert!(content.contains("funding_rate"));
    assert!(content.contains("funding_pnl"));
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}

/// Test that EnhancedCsvExportExt trait is implemented correctly
#[test]
fn test_enhanced_csv_export_ext_trait() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array with constant long position
    let positions = vec![1.0; data.len()];
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Create a temporary file path for testing
    let temp_file = "test_extended_export.csv";
    
    // Export to CSV with extended data
    backtest.export_to_csv_extended(temp_file, true, true, true)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains expected headers
    assert!(content.contains("timestamp"));
    assert!(content.contains("price"));
    assert!(content.contains("position"));
    assert!(content.contains("equity"));
    assert!(content.contains("funding_rate"));
    assert!(content.contains("funding_pnl"));
    assert!(content.contains("trading_pnl"));
    assert!(content.contains("total_pnl"));
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}

/// Test that funding payments can be exported to CSV
#[test]
fn test_export_funding_payments() -> Result<()> {
    // Create test data
    let payments = generate_mock_funding_payments(72, 1.0);
    
    // Create a temporary file path for testing
    let temp_file = "test_funding_payments.csv";
    
    // Export funding payments to CSV
    export_funding_payments_to_csv(&payments, temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains expected headers
    assert!(content.contains("timestamp"));
    assert!(content.contains("funding_rate"));
    assert!(content.contains("position_size"));
    assert!(content.contains("price"));
    assert!(content.contains("payment_amount"));
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}

/// Test that strategy comparison data can be exported to CSV
#[test]
fn test_export_strategy_comparison() -> Result<()> {
    // Create test data for two strategies
    let data = generate_mock_data("BTC", 72, true, false);
    let strategy1_name = "Strategy 1".to_string();
    let strategy2_name = "Strategy 2".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest1 = HyperliquidBacktest::new(
        data.clone(),
        strategy1_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    let mut backtest2 = HyperliquidBacktest::new(
        data.clone(),
        strategy2_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtests
    backtest1.initialize_base_backtest()?;
    backtest2.initialize_base_backtest()?;
    
    // Create position arrays
    let positions1 = generate_position_sequence(data.len(), "constant_long");
    let positions2 = generate_position_sequence(data.len(), "alternating");
    
    // Calculate with funding and positions
    backtest1.calculate_with_funding_and_positions(&positions1)?;
    backtest2.calculate_with_funding_and_positions(&positions2)?;
    
    // Create strategy comparison data
    let comparison = StrategyComparisonData {
        strategies: vec![backtest1, backtest2],
    };
    
    // Create a temporary file path for testing
    let temp_file = "test_strategy_comparison.csv";
    
    // Export strategy comparison to CSV
    comparison.export_to_csv(temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains expected headers
    assert!(content.contains("timestamp"));
    assert!(content.contains("price"));
    assert!(content.contains("Strategy 1_position"));
    assert!(content.contains("Strategy 1_equity"));
    assert!(content.contains("Strategy 2_position"));
    assert!(content.contains("Strategy 2_equity"));
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}

/// Test that funding rate history can be exported to CSV
#[test]
fn test_export_funding_rate_history() -> Result<()> {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    
    // Create a temporary file path for testing
    let temp_file = "test_funding_history.csv";
    
    // Export funding rate history to CSV
    export_funding_rate_history(&data, temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains expected headers
    assert!(content.contains("timestamp"));
    assert!(content.contains("funding_rate"));
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}

/// Test that CSV export handles edge cases correctly
#[test]
fn test_csv_export_edge_cases() -> Result<()> {
    // Test with empty data
    let empty_data = HyperliquidData {
        symbol: "BTC".to_string(),
        datetime: Vec::new(),
        open: Vec::new(),
        high: Vec::new(),
        low: Vec::new(),
        close: Vec::new(),
        volume: Vec::new(),
        funding_rates: Vec::new(),
    };
    
    let strategy_name = "Empty Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut empty_backtest = HyperliquidBacktest::new(
        empty_data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
    empty_backtest.initialize_base_backtest()?;
    
    // Create a temporary file path for testing
    let temp_file = "test_empty_export.csv";
    
    // Export to CSV
    empty_backtest.export_to_csv(temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains headers but no data rows
    assert!(content.contains("timestamp"));
    assert_eq!(content.lines().count(), 1); // Only header line
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    // Test with empty funding payments
    let empty_payments: Vec<FundingPayment> = Vec::new();
    let temp_file = "test_empty_payments.csv";
    
    // Export empty funding payments to CSV
    export_funding_payments_to_csv(&empty_payments, temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains headers but no data rows
    assert!(content.contains("timestamp"));
    assert_eq!(content.lines().count(), 1); // Only header line
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}

/// Test that CSV export handles invalid file paths
#[test]
fn test_csv_export_invalid_path() {
    // Create test data
    let data = generate_mock_data("BTC", 72, true, false);
    let strategy_name = "Test Strategy".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
    backtest.initialize_base_backtest().unwrap();
    
    // Try to export to an invalid path
    let invalid_path = "/invalid/path/that/does/not/exist/file.csv";
    let result = backtest.export_to_csv(invalid_path);
    
    // Should fail with IO error
    assert!(result.is_err());
    if let Err(e) = result {
        match e {
            crate::errors::HyperliquidBacktestError::Io(_) => {}, // Expected
            _ => panic!("Expected IO error"),
        }
    }
}

/// Test that CSV export handles special characters correctly
#[test]
fn test_csv_export_special_characters() -> Result<()> {
    // Create test data with special characters in strategy name
    let data = generate_mock_data("BTC", 10, true, false);
    let strategy_name = "Strategy with, special \"characters\"".to_string();
    let initial_capital = 10000.0;
    let commission = HyperliquidCommission::default();
    
    let mut backtest = HyperliquidBacktest::new(
        data.clone(),
        strategy_name.clone(),
        initial_capital,
        commission.clone(),
    );
    
    // Initialize base backtest
    backtest.initialize_base_backtest()?;
    
    // Create position array
    let positions = vec![1.0; data.len()];
    
    // Calculate with funding and positions
    backtest.calculate_with_funding_and_positions(&positions)?;
    
    // Create a temporary file path for testing
    let temp_file = "test_special_chars.csv";
    
    // Export to CSV
    backtest.export_to_csv(temp_file)?;
    
    // Verify that the file was created
    assert!(Path::new(temp_file).exists());
    
    // Read the file content
    let mut file = fs::File::open(temp_file)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    
    // Verify that the file contains data
    assert!(content.contains("timestamp"));
    assert!(content.lines().count() > 1); // Header + data rows
    
    // Clean up the test file
    fs::remove_file(temp_file)?;
    
    Ok(())
}