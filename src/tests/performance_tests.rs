//! Performance and stress tests for Hyperliquid backtesting system
//! 
//! These tests focus on performance characteristics, memory usage patterns,
//! and system behavior under stress conditions.

use crate::prelude::*;
use chrono::{DateTime, FixedOffset};
use std::time::{Duration, Instant};
use memory_stats::memory_stats;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures::future::join_all;
use tracing::{info, warn};

/// Performance test configuration
struct PerformanceConfig {
    max_execution_time: Duration,
    max_memory_per_point: usize,
    max_total_memory: usize,
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            max_execution_time: Duration::from_secs(30),
            max_memory_per_point: 1000, // bytes per data point
            max_total_memory: 1_000_000_000, // 1GB
        }
    }
}

/// Performance measurement utility
struct PerformanceMeasurement {
    start_time: Instant,
    start_memory: Option<usize>,
    config: PerformanceConfig,
}

impl PerformanceMeasurement {
    fn new(config: PerformanceConfig) -> Self {
        let start_memory = memory_stats().map(|stats| stats.physical_mem);
        Self {
            start_time: Instant::now(),
            start_memory,
            config,
        }
    }

    fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    fn memory_usage(&self) -> Option<usize> {
        if let (Some(start), Some(current_stats)) = (self.start_memory, memory_stats()) {
            Some(current_stats.physical_mem.saturating_sub(start))
        } else {
            None
        }
    }

    fn assert_performance(&self, operation: &str, data_points: usize) {
        let elapsed = self.elapsed();
        assert!(elapsed <= self.config.max_execution_time,
            "{} took {:?}, exceeding limit of {:?}", operation, elapsed, self.config.max_execution_time);

        if let Some(memory_used) = self.memory_usage() {
            assert!(memory_used <= self.config.max_total_memory,
                "{} used {} bytes, exceeding limit of {} bytes", operation, memory_used, self.config.max_total_memory);

            if data_points > 0 {
                let memory_per_point = memory_used / data_points;
                assert!(memory_per_point <= self.config.max_memory_per_point,
                    "{} used {} bytes per data point, exceeding limit of {} bytes", 
                    operation, memory_per_point, self.config.max_memory_per_point);
            }
        }

        info!("{} completed in {:?} with {} data points", operation, elapsed, data_points);
    }
}

#[tokio::test]
async fn test_data_conversion_performance() {
    let sizes = vec![1_000, 10_000, 100_000, 1_000_000];
    
    for size in sizes {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        // Create test data
        let data = create_performance_test_data("BTC", size);
        
        // Test conversion performance
        let _rs_data = data.to_rs_backtester_data();
        
        perf.assert_performance(&format!("Data conversion ({})", size), size);
    }
}

#[tokio::test]
async fn test_backtesting_performance_scaling() {
    let test_cases = vec![
        (1_000, Duration::from_secs(5)),
        (10_000, Duration::from_secs(15)),
        (100_000, Duration::from_secs(60)),
    ];
    
    for (size, max_time) in test_cases {
        let config = PerformanceConfig {
            max_execution_time: max_time,
            ..Default::default()
        };
        let perf = PerformanceMeasurement::new(config);
        
        let data = create_performance_test_data("BTC", size);
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        
        perf.assert_performance(&format!("Backtesting ({})", size), size);
    }
}

#[tokio::test]
async fn test_funding_calculation_performance() {
    let sizes = vec![10_000, 50_000, 100_000];
    
    for size in sizes {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        let data = create_performance_test_data("BTC", size);
        let strategy = funding_arbitrage_strategy(0.001, Default::default());
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        // Focus on funding calculation performance
        backtest.calculate_with_funding();
        let _funding_report = backtest.funding_report();
        
        perf.assert_performance(&format!("Funding calculations ({})", size), size);
    }
}

#[tokio::test]
async fn test_concurrent_backtesting_performance() {
    let num_tasks = 8;
    let data_size = 10_000;
    
    let perf = PerformanceMeasurement::new(PerformanceConfig {
        max_execution_time: Duration::from_secs(45),
        max_total_memory: 2_000_000_000, // 2GB for concurrent operations
        ..Default::default()
    });
    
    let tasks: Vec<_> = (0..num_tasks).map(|i| {
        tokio::spawn(async move {
            let data = create_performance_test_data(&format!("ASSET{}", i), data_size);
            let strategy = enhanced_sma_cross(10 + i, 30 + i * 2, 0.1 + i as f64 * 0.05);
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            
            (i, report.total_return)
        })
    }).collect();
    
    let results = join_all(tasks).await;
    
    // Verify all tasks completed successfully
    for result in results {
        let (task_id, total_return) = result.unwrap();
        assert!(total_return.is_finite(), "Task {} produced invalid return", task_id);
    }
    
    perf.assert_performance("Concurrent backtesting", num_tasks * data_size);
}

#[tokio::test]
async fn test_memory_efficiency_large_datasets() {
    // Test memory efficiency with very large datasets
    let size = 500_000;
    
    let config = PerformanceConfig {
        max_memory_per_point: 500, // Stricter memory limit
        max_total_memory: 500_000_000, // 500MB
        ..Default::default()
    };
    let perf = PerformanceMeasurement::new(config);
    
    let data = create_performance_test_data("BTC", size);
    
    // Test that we can process large datasets efficiently
    let rs_data = data.to_rs_backtester_data();
    let strategy = enhanced_sma_cross(50, 200, 0.2);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    
    backtest.calculate_with_funding();
    let _report = backtest.enhanced_report();
    
    perf.assert_performance("Large dataset processing", size);
}

#[tokio::test]
async fn test_csv_export_performance() {
    let sizes = vec![10_000, 50_000, 100_000];
    
    for size in sizes {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        let data = create_performance_test_data("BTC", size);
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        
        // Test CSV export performance
        let mut csv_buffer = Vec::new();
        backtest.enhanced_csv_export(&mut csv_buffer).unwrap();
        
        assert!(!csv_buffer.is_empty());
        perf.assert_performance(&format!("CSV export ({})", size), size);
    }
}

#[tokio::test]
async fn test_strategy_comparison_performance() {
    let data_size = 20_000;
    let data = create_performance_test_data("BTC", data_size);
    
    let strategies = vec![
        ("SMA Cross 10/20", enhanced_sma_cross(10, 20, 0.0)),
        ("SMA Cross 20/50", enhanced_sma_cross(20, 50, 0.0)),
        ("Funding Aware SMA", enhanced_sma_cross(15, 35, 0.5)),
        ("Funding Arbitrage", funding_arbitrage_strategy(0.001, Default::default())),
    ];
    
    for (strategy_name, strategy) in strategies {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        
        perf.assert_performance(&format!("Strategy: {}", strategy_name), data_size);
    }
}

#[tokio::test]
async fn test_stress_testing_extreme_conditions() {
    // Test system behavior under extreme conditions
    let extreme_scenarios = vec![
        ("High Frequency Data", create_high_frequency_data(50_000)),
        ("Extreme Volatility", create_extreme_volatility_data(20_000)),
        ("Long Time Series", create_long_time_series_data(200_000)),
    ];
    
    for (scenario_name, data) in extreme_scenarios {
        let config = PerformanceConfig {
            max_execution_time: Duration::from_secs(120), // Allow more time for extreme scenarios
            ..Default::default()
        };
        let perf = PerformanceMeasurement::new(config);
        
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let report = backtest.enhanced_report();
        
        // Verify system handles extreme conditions gracefully
        assert!(report.total_return.is_finite(), 
            "Scenario {} produced invalid return", scenario_name);
        assert!(report.max_drawdown.is_finite(),
            "Scenario {} produced invalid drawdown", scenario_name);
        
        perf.assert_performance(&format!("Stress test: {}", scenario_name), data.datetime.len());
    }
}

#[tokio::test]
async fn test_memory_leak_detection_extended() {
    let iterations = 20;
    let data_size = 10_000;
    
    let initial_memory = memory_stats().map(|stats| stats.physical_mem).unwrap_or(0);
    let mut max_memory_growth = 0;
    
    for iteration in 0..iterations {
        let data = create_performance_test_data("BTC", data_size);
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        let _funding_report = backtest.funding_report();
        
        // Export to CSV
        let mut csv_buffer = Vec::new();
        let _ = backtest.enhanced_csv_export(&mut csv_buffer);
        
        // Explicit cleanup
        drop(backtest);
        drop(csv_buffer);
        
        // Check memory growth
        if let Some(current_stats) = memory_stats() {
            let current_memory = current_stats.physical_mem;
            let growth = current_memory.saturating_sub(initial_memory);
            max_memory_growth = max_memory_growth.max(growth);
            
            // Log memory usage every 5 iterations
            if iteration % 5 == 0 {
                info!("Iteration {}: Memory growth = {} bytes", iteration, growth);
            }
        }
        
        // Force garbage collection attempt
        tokio::task::yield_now().await;
    }
    
    // Memory growth should be bounded
    let max_allowed_growth = initial_memory / 5; // 20% of initial memory
    assert!(max_memory_growth < max_allowed_growth,
        "Memory leak detected: {} bytes growth exceeds {} bytes limit", 
        max_memory_growth, max_allowed_growth);
    
    info!("Memory leak test completed. Max growth: {} bytes", max_memory_growth);
}

#[tokio::test]
async fn test_api_response_processing_performance() {
    // Test performance of processing API responses
    let response_sizes = vec![100, 1000, 10000, 50000];
    
    for size in response_sizes {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        // Simulate API response processing
        let mock_response = create_mock_api_response(size);
        let processed_data = process_mock_response(mock_response);
        
        assert_eq!(processed_data.len(), size);
        perf.assert_performance(&format!("API response processing ({})", size), size);
    }
}

#[tokio::test]
async fn test_funding_calculation_edge_cases() {
    // Test funding calculations with edge cases
    let edge_cases = vec![
        ("Zero funding rates", create_zero_funding_data(1000)),
        ("Negative funding rates", create_negative_funding_data(1000)),
        ("Extreme funding rates", create_extreme_funding_data(1000)),
        ("Sparse funding data", create_sparse_funding_data(1000)),
    ];
    
    for (case_name, data) in edge_cases {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        let strategy = funding_arbitrage_strategy(0.001, Default::default());
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let report = backtest.funding_report();
        
        // Verify results are valid even for edge cases
        assert!(report.total_funding_paid.is_finite());
        assert!(report.total_funding_received.is_finite());
        
        perf.assert_performance(&format!("Funding edge case: {}", case_name), 1000);
    }
}

#[tokio::test]
async fn test_concurrent_strategy_execution() {
    // Test concurrent execution of different strategies
    let data = create_performance_test_data("BTC", 5000);
    let num_strategies = 10;
    
    let perf = PerformanceMeasurement::new(PerformanceConfig {
        max_execution_time: Duration::from_secs(60),
        ..Default::default()
    });
    
    let tasks: Vec<_> = (0..num_strategies).map(|i| {
        let data = data.clone();
        tokio::spawn(async move {
            let strategy = enhanced_sma_cross(5 + i, 20 + i * 2, 0.1 + i as f64 * 0.05);
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            
            (i, report.total_return, report.max_drawdown)
        })
    }).collect();
    
    let results = join_all(tasks).await;
    
    // Verify all strategies completed successfully
    for result in results {
        let (strategy_id, total_return, max_drawdown) = result.unwrap();
        assert!(total_return.is_finite(), "Strategy {} produced invalid return", strategy_id);
        assert!(max_drawdown.is_finite(), "Strategy {} produced invalid drawdown", strategy_id);
    }
    
    perf.assert_performance("Concurrent strategy execution", num_strategies * 5000);
}

#[tokio::test]
async fn test_data_structure_performance() {
    // Test performance of data structure operations
    let sizes = vec![1000, 10000, 100000];
    
    for size in sizes {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        // Test data creation performance
        let data = create_performance_test_data("BTC", size);
        
        // Test cloning performance
        let _cloned_data = data.clone();
        
        // Test field access performance
        let _sum: f64 = data.close.iter().sum();
        let _avg = _sum / data.close.len() as f64;
        
        // Test funding rate lookup performance
        for i in (0..size).step_by(100) {
            let _rate = data.get_funding_rate_at(data.datetime[i]);
        }
        
        perf.assert_performance(&format!("Data structure operations ({})", size), size);
    }
}

#[tokio::test]
async fn test_report_generation_performance() {
    // Test performance of different report types
    let data = create_performance_test_data("BTC", 10000);
    let strategy = enhanced_sma_cross(20, 50, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    backtest.calculate_with_funding();
    
    let report_types = vec![
        ("Enhanced Report", || backtest.enhanced_report()),
        ("Funding Report", || backtest.funding_report()),
    ];
    
    for (report_name, report_fn) in report_types {
        let perf = PerformanceMeasurement::new(PerformanceConfig::default());
        
        // Generate report multiple times to test consistency
        for _ in 0..10 {
            let _report = report_fn();
        }
        
        perf.assert_performance(&format!("{} generation", report_name), 10000);
    }
}

#[tokio::test]
async fn test_memory_fragmentation() {
    // Test memory fragmentation with repeated allocations/deallocations
    let initial_memory = memory_stats().map(|stats| stats.physical_mem).unwrap_or(0);
    
    for cycle in 0..20 {
        // Allocate large dataset
        let data = create_performance_test_data("BTC", 50000);
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        
        // Deallocate
        drop(backtest);
        
        // Check memory usage every 5 cycles
        if cycle % 5 == 0 {
            if let Some(current_stats) = memory_stats() {
                let current_memory = current_stats.physical_mem;
                let growth = current_memory.saturating_sub(initial_memory);
                info!("Cycle {}: Memory growth = {} bytes", cycle, growth);
                
                // Memory growth should be bounded
                assert!(growth < initial_memory / 2, 
                       "Excessive memory fragmentation detected: {} bytes", growth);
            }
        }
        
        // Force garbage collection attempt
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn test_cpu_intensive_operations() {
    // Test CPU-intensive operations performance
    let data = create_performance_test_data("BTC", 20000);
    
    let cpu_intensive_operations = vec![
        ("Complex SMA Strategy", enhanced_sma_cross(5, 200, 0.8)),
        ("Funding Arbitrage", funding_arbitrage_strategy(0.0001, Default::default())),
    ];
    
    for (operation_name, strategy) in cpu_intensive_operations {
        let perf = PerformanceMeasurement::new(PerformanceConfig {
            max_execution_time: Duration::from_secs(120), // Allow more time for CPU-intensive ops
            ..Default::default()
        });
        
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let _report = backtest.enhanced_report();
        
        perf.assert_performance(&format!("CPU intensive: {}", operation_name), 20000);
    }
}

#[tokio::test]
async fn test_io_intensive_operations() {
    // Test I/O intensive operations (CSV export, etc.)
    let data = create_performance_test_data("BTC", 50000);
    let strategy = enhanced_sma_cross(20, 50, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    backtest.calculate_with_funding();
    
    let perf = PerformanceMeasurement::new(PerformanceConfig::default());
    
    // Test multiple CSV exports
    for i in 0..5 {
        let mut csv_buffer = Vec::new();
        backtest.enhanced_csv_export(&mut csv_buffer).unwrap();
        assert!(!csv_buffer.is_empty());
        info!("CSV export {} completed, size: {} bytes", i, csv_buffer.len());
    }
    
    perf.assert_performance("I/O intensive operations", 50000);
}

// ============================================================================
// ADDITIONAL HELPER FUNCTIONS FOR PERFORMANCE TESTING
// ============================================================================

/// Create mock API response for performance testing
fn create_mock_api_response(size: usize) -> Vec<serde_json::Value> {
    (0..size).map(|i| {
        serde_json::json!({
            "T": 1640995200000i64 + i as i64 * 3600000,
            "c": format!("{:.2}", 47000.0 + (i as f64 * 0.1).sin() * 100.0),
            "h": format!("{:.2}", 47050.0 + (i as f64 * 0.1).sin() * 100.0),
            "l": format!("{:.2}", 46950.0 + (i as f64 * 0.1).sin() * 100.0),
            "n": 1000 + i,
            "o": format!("{:.2}", 47000.0 + (i as f64 * 0.1).sin() * 100.0 - 10.0),
            "t": 1640995200000i64 + i as i64 * 3600000,
            "v": format!("{:.2}", 100.0 + (i as f64 * 0.05).sin() * 20.0)
        })
    }).collect()
}

/// Process mock API response
fn process_mock_response(response: Vec<serde_json::Value>) -> Vec<(i64, f64, f64, f64, f64, f64)> {
    response.iter().map(|item| {
        (
            item["T"].as_i64().unwrap(),
            item["c"].as_str().unwrap().parse::<f64>().unwrap(),
            item["h"].as_str().unwrap().parse::<f64>().unwrap(),
            item["l"].as_str().unwrap().parse::<f64>().unwrap(),
            item["o"].as_str().unwrap().parse::<f64>().unwrap(),
            item["v"].as_str().unwrap().parse::<f64>().unwrap(),
        )
    }).collect()
}

/// Create data with zero funding rates
fn create_zero_funding_data(size: usize) -> HyperliquidData {
    let mut data = create_performance_test_data("BTC", size);
    data.funding_rates = vec![0.0; size];
    data
}

/// Create data with negative funding rates
fn create_negative_funding_data(size: usize) -> HyperliquidData {
    let mut data = create_performance_test_data("BTC", size);
    data.funding_rates = (0..size).map(|i| -0.0001 - (i as f64 * 0.01).sin() * 0.0001).collect();
    data
}

/// Create data with extreme funding rates
fn create_extreme_funding_data(size: usize) -> HyperliquidData {
    let mut data = create_performance_test_data("BTC", size);
    data.funding_rates = (0..size).map(|i| {
        if i % 100 == 0 { 0.01 } else { 0.0001 } // 1% spikes every 100 periods
    }).collect();
    data
}

/// Create data with sparse funding data
fn create_sparse_funding_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.1).sin() * 100.0)
        .collect();

    // Sparse funding data - only every 8th hour (typical funding interval)
    let funding_size = size / 8;
    let funding_timestamps: Vec<DateTime<FixedOffset>> = (0..funding_size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 8 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime,
        open: prices.iter().map(|p| p - 10.0).collect(),
        high: prices.iter().map(|p| p + 50.0).collect(),
        low: prices.iter().map(|p| p - 50.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: vec![0.0001; funding_size],
        funding_timestamps,
    }
}

#[tokio::test]
async fn test_extreme_load_performance() {
    // Test system behavior under extreme load conditions
    let concurrent_tasks = 50;
    let data_size_per_task = 5_000;
    
    let start_time = std::time::Instant::now();
    let tasks: Vec<_> = (0..concurrent_tasks).map(|i| {
        tokio::spawn(async move {
            let data = create_performance_test_data(&format!("ASSET{}", i), data_size_per_task);
            let strategy = enhanced_sma_cross(10 + (i % 10), 30 + (i % 20), 0.1 + (i as f64 % 5.0) * 0.1);
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            
            backtest.calculate_with_funding();
            let report = backtest.enhanced_report();
            
            (i, report.total_return, report.max_drawdown)
        })
    }).collect();
    
    let results = join_all(tasks).await;
    let total_time = start_time.elapsed();
    
    // Verify all tasks completed successfully
    let mut successful_tasks = 0;
    for result in results {
        if let Ok((task_id, total_return, max_drawdown)) = result {
            if total_return.is_finite() && max_drawdown.is_finite() {
                successful_tasks += 1;
            }
        }
    }
    
    info!("Extreme load test: {}/{} tasks successful in {:?}", 
          successful_tasks, concurrent_tasks, total_time);
    
    // Should complete most tasks successfully within reasonable time
    assert!(successful_tasks >= concurrent_tasks * 9 / 10, "Too many failed tasks under extreme load");
    assert!(total_time.as_secs() < 120, "Extreme load test took too long: {:?}", total_time);
}

#[tokio::test]
async fn test_data_processing_throughput() {
    // Test data processing throughput with various data sizes
    let test_cases = vec![
        (1_000, "Small dataset"),
        (10_000, "Medium dataset"),
        (100_000, "Large dataset"),
        (1_000_000, "Very large dataset"),
    ];
    
    for (size, description) in test_cases {
        let start_time = std::time::Instant::now();
        
        // Create data
        let data = create_performance_test_data("BTC", size);
        let creation_time = start_time.elapsed();
        
        // Convert data
        let conversion_start = std::time::Instant::now();
        let _rs_data = data.to_rs_backtester_data();
        let conversion_time = conversion_start.elapsed();
        
        // Calculate throughput
        let creation_throughput = size as f64 / creation_time.as_secs_f64();
        let conversion_throughput = size as f64 / conversion_time.as_secs_f64();
        
        info!("{}: Creation throughput: {:.0} points/sec, Conversion throughput: {:.0} points/sec", 
              description, creation_throughput, conversion_throughput);
        
        // Verify reasonable throughput (at least 1000 points per second)
        assert!(creation_throughput > 1000.0, 
            "Data creation throughput too low: {:.0} points/sec", creation_throughput);
        assert!(conversion_throughput > 1000.0, 
            "Data conversion throughput too low: {:.0} points/sec", conversion_throughput);
    }
}

#[tokio::test]
async fn test_funding_calculation_scalability() {
    // Test funding calculation performance with different dataset sizes
    let sizes = vec![1_000, 10_000, 50_000, 100_000];
    let mut performance_metrics = Vec::new();
    
    for size in sizes {
        let data = create_performance_test_data("BTC", size);
        let strategy = funding_arbitrage_strategy(0.001, Default::default());
        
        let start_time = std::time::Instant::now();
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        backtest.calculate_with_funding();
        let calculation_time = start_time.elapsed();
        
        let throughput = size as f64 / calculation_time.as_secs_f64();
        performance_metrics.push((size, throughput));
        
        info!("Funding calculation for {} points: {:.2}s ({:.0} points/sec)", 
              size, calculation_time.as_secs_f64(), throughput);
        
        // Verify reasonable performance
        assert!(throughput > 100.0, 
            "Funding calculation throughput too low: {:.0} points/sec for {} points", 
            throughput, size);
    }
    
    // Verify scalability - throughput shouldn't degrade significantly with size
    if performance_metrics.len() >= 2 {
        let (small_size, small_throughput) = performance_metrics[0];
        let (large_size, large_throughput) = performance_metrics[performance_metrics.len() - 1];
        
        let throughput_ratio = large_throughput / small_throughput;
        info!("Throughput scaling: {:.2}x from {} to {} points", 
              throughput_ratio, small_size, large_size);
        
        // Throughput shouldn't degrade by more than 50%
        assert!(throughput_ratio > 0.5, 
            "Funding calculation throughput degraded too much: {:.2}x", throughput_ratio);
    }
}

#[tokio::test]
async fn test_memory_pressure_handling() {
    // Test system behavior under memory pressure
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    
    let memory_pressure_flag = Arc::new(AtomicBool::new(false));
    let initial_memory = memory_stats().map(|stats| stats.physical_mem).unwrap_or(0);
    
    // Create progressively larger datasets until memory pressure
    let mut current_size = 10_000;
    let mut successful_iterations = 0;
    
    for iteration in 0..10 {
        let data = create_performance_test_data("BTC", current_size);
        
        // Check memory usage
        if let Some(current_stats) = memory_stats() {
            let current_memory = current_stats.physical_mem;
            let memory_usage = current_memory.saturating_sub(initial_memory);
            
            // If memory usage exceeds 1GB, we're under pressure
            if memory_usage > 1_000_000_000 {
                memory_pressure_flag.store(true, Ordering::Relaxed);
                info!("Memory pressure detected at iteration {} with {} points", iteration, current_size);
                break;
            }
        }
        
        // Try to process the data
        let strategy = enhanced_sma_cross(20, 50, 0.3);
        let processing_result = std::panic::catch_unwind(|| {
            let mut backtest = HyperliquidBacktest::new(
                data,
                strategy,
                10000.0,
                HyperliquidCommission::default(),
            );
            backtest.calculate_with_funding();
            backtest.enhanced_report()
        });
        
        match processing_result {
            Ok(report) => {
                if report.total_return.is_finite() {
                    successful_iterations += 1;
                    current_size += 10_000; // Increase size for next iteration
                } else {
                    break;
                }
            }
            Err(_) => {
                info!("Processing failed at iteration {} with {} points", iteration, current_size);
                break;
            }
        }
    }
    
    info!("Memory pressure test: {} successful iterations, max size: {}", 
          successful_iterations, current_size - 10_000);
    
    // Should handle at least a few iterations before hitting limits
    assert!(successful_iterations >= 3, 
        "System failed too early under memory pressure: {} iterations", successful_iterations);
}

// ============================================================================
// HELPER FUNCTIONS FOR PERFORMANCE TESTING
// ============================================================================

/// Create performance test data with realistic characteristics
fn create_performance_test_data(ticker: &str, size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let base_price = match ticker {
        "BTC" => 47000.0,
        "ETH" => 3500.0,
        "SOL" => 150.0,
        _ => 100.0,
    };

    // Generate realistic price data with trends and volatility
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            let t = i as f64 / size as f64;
            let trend = base_price * 0.2 * t; // 20% trend over the period
            let cycle = (i as f64 * 0.01).sin() * base_price * 0.05; // 5% cyclical movement
            let noise = (i as f64 * 0.1).sin() * base_price * 0.01; // 1% noise
            base_price + trend + cycle + noise
        })
        .collect();

    HyperliquidData {
        ticker: ticker.to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + p * 0.005).collect(),
        low: prices.iter().map(|p| p - p * 0.005).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.05).sin() * 20.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.02).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create high frequency data for stress testing
fn create_high_frequency_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 60, 0) // 1-minute intervals
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let mut price = 47000.0;
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            // High frequency price changes
            let change = (i as f64 * 0.5).sin() * 10.0 + (i as f64 * 0.3).cos() * 5.0;
            price += change;
            price
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 5.0).collect(),
        low: prices.iter().map(|p| p - 5.0).collect(),
        close: prices,
        volume: vec![50.0; size], // Consistent volume for high frequency
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}

/// Create extreme volatility data for stress testing
fn create_extreme_volatility_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let mut price = 47000.0;
    let prices: Vec<f64> = (0..size)
        .map(|i| {
            // Extreme volatility with large price swings
            let volatility = (i as f64 * 0.1).sin() * price * 0.2; // 20% swings
            let shock = if i % 100 == 0 { (i as f64).sin() * price * 0.1 } else { 0.0 }; // Periodic shocks
            price += volatility + shock;
            price.max(1000.0) // Prevent negative prices
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + p * 0.05).collect(),
        low: prices.iter().map(|p| p - p * 0.05).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.2).sin().abs() * 200.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.05).sin() * 0.0005).collect(),
        funding_timestamps: datetime,
    }
}

/// Create long time series data for memory testing
fn create_long_time_series_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    // Simple but long price series
    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.001).sin() * 1000.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().enumerate().map(|(i, p)| {
            if i == 0 { *p } else { prices[i-1] }
        }).collect(),
        high: prices.iter().map(|p| p + 10.0).collect(),
        low: prices.iter().map(|p| p - 10.0).collect(),
        close: prices,
        volume: vec![100.0; size],
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}