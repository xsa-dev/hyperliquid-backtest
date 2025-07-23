//! Tests for the enhanced CSV export functionality

use crate::data::HyperliquidData;
use crate::funding_report::*;
use crate::csv_export::*;
use crate::errors::Result;
use chrono::{DateTime, FixedOffset, TimeZone};
use std::fs;
use std::path::Path;

fn create_test_datetime(timestamp: i64) -> DateTime<FixedOffset> {
    FixedOffset::east_opt(0).unwrap().timestamp_opt(timestamp, 0).unwrap()
}

fn create_test_data() -> HyperliquidData {
    let datetime = vec![
        create_test_datetime(1640995200), // 2022-01-01 00:00:00 UTC
        create_test_datetime(1640998800), // 2022-01-01 01:00:00 UTC
        create_test_datetime(1641002400), // 2022-01-01 02:00:00 UTC
        create_test_datetime(1641006000), // 2022-01-01 03:00:00 UTC
        create_test_datetime(1641009600), // 2022-01-01 04:00:00 UTC
    ];
    
    let open = vec![100.0, 101.0, 102.0, 103.0, 104.0];
    let high = vec![105.0, 106.0, 107.0, 108.0, 109.0];
    let low = vec![95.0, 96.0, 97.0, 98.0, 99.0];
    let close = vec![103.0, 104.0, 105.0, 106.0, 107.0];
    let volume = vec![1000.0, 1100.0, 1200.0, 1300.0, 1400.0];
    
    // Create funding data
    let funding_timestamps = vec![
        create_test_datetime(1640995200), // 2022-01-01 00:00:00 UTC
        create_test_datetime(1641024000), // 2022-01-01 08:00:00 UTC
        create_test_datetime(1641052800), // 2022-01-01 16:00:00 UTC
    ];
    
    let funding_rates = vec![0.0001, -0.0002, 0.0003]; // 0.01%, -0.02%, 0.03%
    
    let mut data = HyperliquidData::with_ohlc_data(
        "BTC".to_string(),
        datetime.clone(),
        open,
        high,
        low,
        close,
        volume,
    ).unwrap();
    
    // Add funding data
    data.funding_timestamps = funding_timestamps;
    data.funding_rates = funding_rates;
    
    data
}

fn create_test_funding_report(data: &HyperliquidData) -> FundingReport {
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0]; // Long, then short
    let position_values = vec![103.0, 208.0, 157.5, 0.0, -107.0]; // position_size * close
    let trading_pnl = 50.0;
    let funding_pnl = 5.0;
    
    FundingReport::new(
        "BTC",
        data,
        &position_sizes,
        &position_values,
        trading_pnl,
        funding_pnl,
    ).unwrap()
}

#[test]
fn test_enhanced_csv_export_creation() {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    assert_eq!(export.data.ticker, "BTC");
    assert_eq!(export.trading_pnl.len(), 5);
    assert_eq!(export.funding_pnl.len(), 5);
    assert_eq!(export.total_pnl.len(), 5);
    assert_eq!(export.position_sizes.len(), 5);
    assert!(export.funding_report.is_some());
}

#[test]
fn test_export_to_csv() -> Result<()> {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let test_file = "test_export.csv";
    export.export_to_csv(test_file)?;
    
    // Verify file exists
    assert!(Path::new(test_file).exists());
    
    // Read file content
    let content = fs::read_to_string(test_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Check header
    assert!(content.contains("timestamp,open,high,low,close,volume,funding_rate,position_size,trading_pnl,funding_pnl,total_pnl"));
    
    // Check data rows
    assert!(content.contains("2022-01-01 00:00:00,100,105,95,103,1000,0.00010000,1,0.00,0.00,0.00"));
    assert!(content.contains("2022-01-01 01:00:00,101,106,96,104,1100,0.00000000,2,2.00,0.10,2.10"));
    
    // Clean up
    fs::remove_file(test_file).unwrap();
    
    Ok(())
}

#[test]
fn test_export_funding_history_to_csv() -> Result<()> {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let test_file = "test_funding_history.csv";
    export.export_funding_history_to_csv(test_file)?;
    
    // Verify file exists
    assert!(Path::new(test_file).exists());
    
    // Read file content
    let content = fs::read_to_string(test_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Check header
    assert!(content.contains("timestamp,funding_rate"));
    
    // Check data rows
    assert!(content.contains("2022-01-01 00:00:00,0.00010000"));
    assert!(content.contains("2022-01-01 08:00:00,-0.00020000"));
    assert!(content.contains("2022-01-01 16:00:00,0.00030000"));
    
    // Clean up
    fs::remove_file(test_file).unwrap();
    
    Ok(())
}

#[test]
fn test_export_funding_payments_to_csv() -> Result<()> {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let test_file = "test_funding_payments.csv";
    export.export_funding_payments_to_csv(test_file)?;
    
    // Verify file exists
    assert!(Path::new(test_file).exists());
    
    // Read file content
    let content = fs::read_to_string(test_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Check header
    assert!(content.contains("timestamp,funding_rate,position_size,position_value,payment,cumulative"));
    
    // Clean up
    fs::remove_file(test_file).unwrap();
    
    Ok(())
}

#[test]
fn test_export_funding_statistics_to_csv() -> Result<()> {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let test_file = "test_funding_statistics.csv";
    export.export_funding_statistics_to_csv(test_file)?;
    
    // Verify file exists
    assert!(Path::new(test_file).exists());
    
    // Read file content
    let content = fs::read_to_string(test_file)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    // Check header
    assert!(content.contains("metric,value,description"));
    
    // Check some key metrics
    assert!(content.contains("symbol,BTC,Trading pair"));
    assert!(content.contains("total_funding_paid"));
    assert!(content.contains("total_funding_received"));
    assert!(content.contains("annualized_funding_yield"));
    
    // Clean up
    fs::remove_file(test_file).unwrap();
    
    Ok(())
}

#[test]
fn test_export_all_funding_data() -> Result<()> {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let base_path = "test_export_all";
    let exported_files = export.export_all_funding_data(base_path)?;
    
    // Verify files exist - now we should have more files with the enhanced exports
    assert!(exported_files.len() >= 5); // backtest, funding history, funding payments, funding statistics, funding metrics, etc.
    
    // Check for specific file patterns
    let has_backtest_file = exported_files.iter().any(|f| f.contains("backtest.csv"));
    let has_funding_history_file = exported_files.iter().any(|f| f.contains("funding_history.csv"));
    let has_funding_payments_file = exported_files.iter().any(|f| f.contains("funding_payments.csv"));
    let has_funding_statistics_file = exported_files.iter().any(|f| f.contains("funding_statistics.csv"));
    let has_funding_metrics_file = exported_files.iter().any(|f| f.contains("funding_metrics.csv"));
    
    assert!(has_backtest_file, "Missing backtest CSV file");
    assert!(has_funding_history_file, "Missing funding history CSV file");
    assert!(has_funding_payments_file, "Missing funding payments CSV file");
    assert!(has_funding_statistics_file, "Missing funding statistics CSV file");
    assert!(has_funding_metrics_file, "Missing funding metrics CSV file");
    
    for file in &exported_files {
        assert!(Path::new(file).exists());
        
        // Clean up
        fs::remove_file(file).unwrap();
    }
    
    Ok(())
}

#[test]
fn test_data_length_mismatch_validation() {
    let data = create_test_data();
    let funding_report = create_test_funding_report(&data);
    let trading_pnl = vec![0.0, 2.0, 5.0]; // Too short
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        Some(funding_report),
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let test_file = "test_validation.csv";
    let result = export.export_to_csv(test_file);
    
    // Should fail with validation error
    assert!(result.is_err());
    
    // Clean up if file was created
    if Path::new(test_file).exists() {
        fs::remove_file(test_file).unwrap();
    }
}

#[test]
fn test_export_without_funding_report() {
    let data = create_test_data();
    let trading_pnl = vec![0.0, 2.0, 5.0, 10.0, 15.0];
    let funding_pnl = vec![0.0, 0.1, 0.2, 0.3, 0.4];
    let total_pnl = vec![0.0, 2.1, 5.2, 10.3, 15.4];
    let position_sizes = vec![1.0, 2.0, 1.5, 0.0, -1.0];
    
    let export = EnhancedCsvExport::new(
        data,
        None, // No funding report
        trading_pnl,
        funding_pnl,
        total_pnl,
        position_sizes,
    );
    
    let test_file = "test_no_funding_report.csv";
    
    // Should succeed for regular export
    let result = export.export_to_csv(test_file);
    assert!(result.is_ok());
    
    // Should succeed for funding history export
    let history_file = "test_no_funding_history.csv";
    let result = export.export_funding_history_to_csv(history_file);
    assert!(result.is_ok());
    
    // Should fail for funding payments export
    let payments_file = "test_no_funding_payments.csv";
    let result = export.export_funding_payments_to_csv(payments_file);
    assert!(result.is_err());
    
    // Clean up
    if Path::new(test_file).exists() {
        fs::remove_file(test_file).unwrap();
    }
    if Path::new(history_file).exists() {
        fs::remove_file(history_file).unwrap();
    }
    if Path::new(payments_file).exists() {
        fs::remove_file(payments_file).unwrap();
    }
}