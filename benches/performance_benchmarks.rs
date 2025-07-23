//! Performance benchmarks for data processing
//! 
//! These benchmarks measure the performance of key operations including
//! data conversion, funding calculations, and backtesting workflows.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use hyperliquid_backtester::prelude::*;
use chrono::{DateTime, FixedOffset};
use std::time::Duration;

/// Create test data of specified size
fn create_benchmark_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    let prices: Vec<f64> = (0..size)
        .map(|i| 47000.0 + (i as f64 * 0.1).sin() * 100.0 + (i as f64 * 0.05).cos() * 50.0)
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().map(|p| p - 10.0).collect(),
        high: prices.iter().map(|p| p + 50.0).collect(),
        low: prices.iter().map(|p| p - 50.0).collect(),
        close: prices,
        volume: (0..size).map(|i| 100.0 + (i as f64 * 0.1).sin() * 20.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.01).sin() * 0.0001).collect(),
        funding_timestamps: datetime,
    }
}

/// Benchmark data conversion from HyperliquidData to rs-backtester Data
fn bench_data_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_conversion");
    
    for size in [100, 1000, 10000, 100000].iter() {
        let data = create_benchmark_data(*size);
        
        group.bench_with_input(
            BenchmarkId::new("to_rs_backtester_data", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(data.to_rs_backtester_data())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark funding rate lookups
fn bench_funding_rate_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("funding_rate_lookup");
    
    for size in [100, 1000, 10000, 100000].iter() {
        let data = create_benchmark_data(*size);
        let lookup_timestamp = data.datetime[size / 2]; // Middle timestamp
        
        group.bench_with_input(
            BenchmarkId::new("get_funding_rate_at", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(data.get_funding_rate_at(lookup_timestamp))
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark complete backtesting workflow
fn bench_backtesting_workflow(c: &mut Criterion) {
    let mut group = c.benchmark_group("backtesting_workflow");
    group.measurement_time(Duration::from_secs(10)); // Longer measurement time for complex operations
    
    for size in [100, 1000, 5000].iter() { // Smaller sizes for complex operations
        let data = create_benchmark_data(*size);
        let strategy = enhanced_sma_cross(10, 20, 0.5);
        
        group.bench_with_input(
            BenchmarkId::new("complete_backtest", size),
            size,
            |b, _| {
                b.iter(|| {
                    let mut backtest = HyperliquidBacktest::new(
                        black_box(data.clone()),
                        black_box(strategy.clone()),
                        10000.0,
                        HyperliquidCommission::default(),
                    );
                    backtest.calculate_with_funding();
                    black_box(backtest)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark funding payment calculations
fn bench_funding_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("funding_calculations");
    
    for size in [100, 1000, 10000].iter() {
        let data = create_benchmark_data(*size);
        let strategy = enhanced_sma_cross(10, 20, 0.5);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        
        group.bench_with_input(
            BenchmarkId::new("calculate_with_funding", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(backtest.calculate_with_funding())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark strategy execution
fn bench_strategy_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_execution");
    
    let data = create_benchmark_data(1000);
    let rs_data = data.to_rs_backtester_data();
    
    // Benchmark different strategies
    let strategies = vec![
        ("sma_cross_10_20", enhanced_sma_cross(10, 20, 0.0)),
        ("sma_cross_5_15", enhanced_sma_cross(5, 15, 0.0)),
        ("funding_aware_sma", enhanced_sma_cross(10, 20, 0.5)),
        ("funding_arbitrage", funding_arbitrage_strategy(0.001, Default::default())),
    ];
    
    for (name, strategy) in strategies {
        group.bench_function(name, |b| {
            b.iter(|| {
                use rs_backtester::prelude::*;
                let backtest = Backtest::new(
                    black_box(rs_data.clone()),
                    black_box(strategy.clone()),
                    10000.0,
                    Commission::default(),
                );
                black_box(backtest)
            })
        });
    }
    
    group.finish();
}

/// Benchmark CSV export functionality
fn bench_csv_export(c: &mut Criterion) {
    let mut group = c.benchmark_group("csv_export");
    
    for size in [100, 1000, 10000].iter() {
        let data = create_benchmark_data(*size);
        let strategy = enhanced_sma_cross(10, 20, 0.5);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        backtest.calculate_with_funding();
        
        group.bench_with_input(
            BenchmarkId::new("enhanced_csv_export", size),
            size,
            |b, _| {
                b.iter(|| {
                    let mut csv_data = Vec::new();
                    black_box(backtest.enhanced_csv_export(&mut csv_data).unwrap())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark funding report generation
fn bench_funding_report(c: &mut Criterion) {
    let mut group = c.benchmark_group("funding_report");
    
    for size in [100, 1000, 10000].iter() {
        let data = create_benchmark_data(*size);
        let strategy = enhanced_sma_cross(10, 20, 0.5);
        let mut backtest = HyperliquidBacktest::new(
            data,
            strategy,
            10000.0,
            HyperliquidCommission::default(),
        );
        backtest.calculate_with_funding();
        
        group.bench_with_input(
            BenchmarkId::new("funding_report_generation", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(backtest.funding_report())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark memory-intensive operations
fn bench_memory_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_operations");
    
    // Test cloning large datasets
    for size in [1000, 10000, 50000].iter() {
        let data = create_benchmark_data(*size);
        
        group.bench_with_input(
            BenchmarkId::new("data_clone", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(data.clone())
                })
            },
        );
    }
    
    // Test vector operations
    for size in [1000, 10000, 100000].iter() {
        let prices: Vec<f64> = (0..*size).map(|i| i as f64).collect();
        
        group.bench_with_input(
            BenchmarkId::new("vector_sum", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(prices.iter().sum::<f64>())
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("vector_map", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(prices.iter().map(|x| x * 2.0).collect::<Vec<f64>>())
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark concurrent operations
fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");
    
    // Benchmark parallel data processing
    group.bench_function("parallel_data_creation", |b| {
        b.iter(|| {
            let handles: Vec<_> = (0..4).map(|i| {
                std::thread::spawn(move || {
                    create_benchmark_data(1000 + i * 100)
                })
            }).collect();
            
            let results: Vec<HyperliquidData> = handles.into_iter()
                .map(|h| h.join().unwrap())
                .collect();
            
            black_box(results)
        })
    });
    
    group.finish();
}

/// Benchmark memory allocation patterns
fn bench_memory_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_allocation");
    
    for size in [1000, 10000, 100000].iter() {
        group.bench_with_input(
            BenchmarkId::new("vector_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let _vec: Vec<f64> = (0..size).map(|i| i as f64).collect();
                    black_box(_vec)
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("datetime_allocation", size),
            size,
            |b, &size| {
                b.iter(|| {
                    let _datetime: Vec<DateTime<FixedOffset>> = (0..size)
                        .map(|i| {
                            DateTime::from_timestamp(1640995200 + i as i64, 0)
                                .unwrap()
                                .with_timezone(&FixedOffset::east_opt(0).unwrap())
                        })
                        .collect();
                    black_box(_datetime)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark data structure operations
fn bench_data_structure_ops(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_structure_operations");
    
    let data = create_benchmark_data(10000);
    
    group.bench_function("data_clone", |b| {
        b.iter(|| {
            black_box(data.clone())
        })
    });
    
    group.bench_function("funding_rate_search", |b| {
        let search_timestamp = data.datetime[5000];
        b.iter(|| {
            black_box(data.get_funding_rate_at(search_timestamp))
        })
    });
    
    group.bench_function("price_vector_operations", |b| {
        b.iter(|| {
            let sum: f64 = data.close.iter().sum();
            let avg = sum / data.close.len() as f64;
            let variance: f64 = data.close.iter()
                .map(|x| (x - avg).powi(2))
                .sum::<f64>() / data.close.len() as f64;
            black_box((sum, avg, variance))
        })
    });
    
    group.finish();
}

/// Benchmark API response processing simulation
fn bench_api_response_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_response_processing");
    
    // Simulate processing of different sized API responses
    for size in [100, 1000, 10000].iter() {
        let mock_candles: Vec<serde_json::Value> = (0..*size).map(|i| {
            serde_json::json!({
                "T": 1640995200000i64 + i as i64 * 3600000,
                "c": format!("{:.1}", 47000.0 + i as f64),
                "h": format!("{:.1}", 47050.0 + i as f64),
                "l": format!("{:.1}", 46950.0 + i as f64),
                "n": 1000 + i,
                "o": format!("{:.1}", 47000.0 + i as f64 - 10.0),
                "t": 1640995200000i64 + i as i64 * 3600000,
                "v": format!("{:.1}", 100.0 + i as f64)
            })
        }).collect();
        
        group.bench_with_input(
            BenchmarkId::new("json_parsing", size),
            size,
            |b, _| {
                b.iter(|| {
                    let _parsed: Vec<_> = mock_candles.iter()
                        .map(|candle| {
                            (
                                candle["T"].as_i64().unwrap(),
                                candle["c"].as_str().unwrap().parse::<f64>().unwrap(),
                                candle["h"].as_str().unwrap().parse::<f64>().unwrap(),
                                candle["l"].as_str().unwrap().parse::<f64>().unwrap(),
                                candle["o"].as_str().unwrap().parse::<f64>().unwrap(),
                                candle["v"].as_str().unwrap().parse::<f64>().unwrap(),
                            )
                        })
                        .collect();
                    black_box(_parsed)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark different strategy types
fn bench_strategy_types(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_types");
    
    let data = create_benchmark_data(5000);
    let rs_data = data.to_rs_backtester_data();
    
    // Test different strategy complexities
    let strategies = vec![
        ("simple_sma", enhanced_sma_cross(10, 20, 0.0)),
        ("complex_sma", enhanced_sma_cross(5, 50, 0.3)),
        ("funding_arbitrage", funding_arbitrage_strategy(0.001, Default::default())),
    ];
    
    for (name, strategy) in strategies {
        group.bench_function(name, |b| {
            b.iter(|| {
                use rs_backtester::prelude::*;
                let backtest = Backtest::new(
                    black_box(rs_data.clone()),
                    black_box(strategy.clone()),
                    10000.0,
                    Commission::default(),
                );
                black_box(backtest)
            })
        });
    }
    
    group.finish();
}

/// Benchmark report generation
fn bench_report_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("report_generation");
    
    let data = create_benchmark_data(10000);
    let strategy = enhanced_sma_cross(20, 50, 0.3);
    let mut backtest = HyperliquidBacktest::new(
        data,
        strategy,
        10000.0,
        HyperliquidCommission::default(),
    );
    backtest.calculate_with_funding();
    
    group.bench_function("enhanced_report", |b| {
        b.iter(|| {
            black_box(backtest.enhanced_report())
        })
    });
    
    group.bench_function("funding_report", |b| {
        b.iter(|| {
            black_box(backtest.funding_report())
        })
    });
    
    group.bench_function("csv_export", |b| {
        b.iter(|| {
            let mut buffer = Vec::new();
            black_box(backtest.enhanced_csv_export(&mut buffer).unwrap())
        })
    });
    
    group.finish();
}

/// Benchmark error handling performance
fn bench_error_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("error_handling");
    
    // Test error creation and formatting performance
    group.bench_function("error_creation", |b| {
        b.iter(|| {
            let errors = vec![
                HyperliquidBacktestError::DataConversion("Test error".to_string()),
                HyperliquidBacktestError::InvalidTimeRange { start: 100, end: 50 },
                HyperliquidBacktestError::UnsupportedInterval("invalid".to_string()),
                HyperliquidBacktestError::Backtesting("Test backtest error".to_string()),
            ];
            black_box(errors)
        })
    });
    
    group.bench_function("error_formatting", |b| {
        let error = HyperliquidBacktestError::DataConversion("Test error message".to_string());
        b.iter(|| {
            black_box(error.to_string())
        })
    });
    
    group.finish();
}

/// Benchmark data validation performance
fn bench_data_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_validation");
    
    for size in [1000, 10000, 100000].iter() {
        let data = create_benchmark_data(*size);
        
        group.bench_with_input(
            BenchmarkId::new("ohlc_validation", size),
            size,
            |b, _| {
                b.iter(|| {
                    // Simulate OHLC validation
                    for i in 0..data.close.len() {
                        let _valid = data.high[i] >= data.low[i] 
                            && data.high[i] >= data.open[i] 
                            && data.high[i] >= data.close[i]
                            && data.low[i] <= data.open[i] 
                            && data.low[i] <= data.close[i];
                        black_box(_valid);
                    }
                })
            },
        );
        
        group.bench_with_input(
            BenchmarkId::new("funding_rate_validation", size),
            size,
            |b, _| {
                b.iter(|| {
                    // Simulate funding rate validation
                    for rate in &data.funding_rates {
                        let _valid = rate.is_finite() && rate.abs() < 1.0;
                        black_box(_valid);
                    }
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark large dataset operations
fn bench_large_dataset_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_dataset_operations");
    group.measurement_time(Duration::from_secs(20)); // Longer measurement time
    
    // Test with very large datasets
    for size in [100000, 500000, 1000000].iter() {
        let data = create_benchmark_data(*size);
        
        group.bench_with_input(
            BenchmarkId::new("large_data_conversion", size),
            size,
            |b, _| {
                b.iter(|| {
                    black_box(data.to_rs_backtester_data())
                })
            },
        );
        
        // Only test smaller sizes for full backtesting due to time constraints
        if *size <= 100000 {
            let strategy = enhanced_sma_cross(20, 50, 0.3);
            group.bench_with_input(
                BenchmarkId::new("large_backtest", size),
                size,
                |b, _| {
                    b.iter(|| {
                        let mut backtest = HyperliquidBacktest::new(
                            black_box(data.clone()),
                            black_box(strategy.clone()),
                            10000.0,
                            HyperliquidCommission::default(),
                        );
                        backtest.calculate_with_funding();
                        black_box(backtest)
                    })
                },
            );
        }
    }
    
    group.finish();
}

/// Benchmark memory-intensive concurrent operations
fn bench_memory_intensive_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_intensive_concurrent");
    group.measurement_time(Duration::from_secs(15));
    
    group.bench_function("concurrent_large_datasets", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let tasks: Vec<_> = (0..4).map(|i| {
                    tokio::spawn(async move {
                        let data = create_benchmark_data(25000 + i * 5000);
                        let strategy = enhanced_sma_cross(10 + i, 30 + i * 2, 0.1 + i as f64 * 0.1);
                        let mut backtest = HyperliquidBacktest::new(
                            data,
                            strategy,
                            10000.0,
                            HyperliquidCommission::default(),
                        );
                        backtest.calculate_with_funding();
                        backtest.enhanced_report()
                    })
                }).collect();
                
                let results = futures::future::join_all(tasks).await;
                black_box(results)
            })
        })
    });
    
    group.finish();
}

/// Benchmark API simulation with realistic delays
fn bench_api_simulation_realistic(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_simulation_realistic");
    
    for size in [100, 1000, 5000].iter() {
        group.bench_with_input(
            BenchmarkId::new("realistic_api_processing", size),
            size,
            |b, &size| {
                b.iter(|| {
                    // Simulate realistic API response processing with some delay
                    let mock_response = create_realistic_mock_response(size);
                    let processed = process_realistic_mock_response(mock_response);
                    
                    // Simulate network delay
                    std::thread::sleep(Duration::from_micros(size as u64 / 10));
                    
                    black_box(processed)
                })
            },
        );
    }
    
    group.finish();
}

/// Benchmark funding arbitrage strategy specifically
fn bench_funding_arbitrage_detailed(c: &mut Criterion) {
    let mut group = c.benchmark_group("funding_arbitrage_detailed");
    
    let data = create_benchmark_data(10000);
    
    // Test different funding thresholds
    let thresholds = vec![0.0001, 0.001, 0.01];
    
    for threshold in thresholds {
        let strategy = funding_arbitrage_strategy(threshold, Default::default());
        
        group.bench_function(&format!("threshold_{}", threshold), |b| {
            b.iter(|| {
                let mut backtest = HyperliquidBacktest::new(
                    black_box(data.clone()),
                    black_box(strategy.clone()),
                    10000.0,
                    HyperliquidCommission::default(),
                );
                backtest.calculate_with_funding();
                black_box(backtest.funding_report())
            })
        });
    }
    
    group.finish();
}

/// Benchmark edge case handling
fn bench_edge_cases(c: &mut Criterion) {
    let mut group = c.benchmark_group("edge_cases");
    
    // Test with extreme data scenarios
    let edge_case_data = vec![
        ("zero_funding", create_zero_funding_benchmark_data(1000)),
        ("negative_funding", create_negative_funding_benchmark_data(1000)),
        ("extreme_volatility", create_extreme_volatility_benchmark_data(1000)),
        ("minimal_data", create_minimal_benchmark_data(10)),
    ];
    
    for (case_name, data) in edge_case_data {
        let strategy = enhanced_sma_cross(5, 15, 0.3);
        
        group.bench_function(case_name, |b| {
            b.iter(|| {
                let mut backtest = HyperliquidBacktest::new(
                    black_box(data.clone()),
                    black_box(strategy.clone()),
                    10000.0,
                    HyperliquidCommission::default(),
                );
                backtest.calculate_with_funding();
                black_box(backtest.enhanced_report())
            })
        });
    }
    
    group.finish();
}

// ============================================================================
// ADDITIONAL HELPER FUNCTIONS FOR BENCHMARKING
// ============================================================================

/// Create realistic mock API response with proper structure
fn create_realistic_mock_response(size: usize) -> Vec<serde_json::Value> {
    let mut base_price = 47000.0;
    (0..size).map(|i| {
        // Realistic price movement
        let change = (i as f64 * 0.1).sin() * 50.0 + (i as f64 * 0.05).cos() * 25.0;
        base_price += change;
        
        let open = base_price - 5.0;
        let high = base_price + 25.0;
        let low = base_price - 25.0;
        let close = base_price;
        let volume = 100.0 + (i as f64 * 0.1).sin().abs() * 50.0;
        
        serde_json::json!({
            "T": 1640995200000i64 + i as i64 * 3600000,
            "c": format!("{:.2}", close),
            "h": format!("{:.2}", high),
            "l": format!("{:.2}", low),
            "n": 1000 + i,
            "o": format!("{:.2}", open),
            "t": 1640995200000i64 + i as i64 * 3600000,
            "v": format!("{:.2}", volume)
        })
    }).collect()
}

/// Process realistic mock API response
fn process_realistic_mock_response(response: Vec<serde_json::Value>) -> Vec<(i64, f64, f64, f64, f64, f64)> {
    response.iter().map(|item| {
        // Simulate realistic parsing with error handling
        let timestamp = item["T"].as_i64().unwrap_or(0);
        let close = item["c"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let high = item["h"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let low = item["l"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let open = item["o"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        let volume = item["v"].as_str().unwrap_or("0").parse::<f64>().unwrap_or(0.0);
        
        (timestamp, close, high, low, open, volume)
    }).collect()
}

/// Create benchmark data with zero funding rates
fn create_zero_funding_benchmark_data(size: usize) -> HyperliquidData {
    let mut data = create_benchmark_data(size);
    data.funding_rates = vec![0.0; size];
    data
}

/// Create benchmark data with negative funding rates
fn create_negative_funding_benchmark_data(size: usize) -> HyperliquidData {
    let mut data = create_benchmark_data(size);
    data.funding_rates = (0..size).map(|i| -0.0001 - (i as f64 * 0.01).sin() * 0.0001).collect();
    data
}

/// Create benchmark data with extreme volatility
fn create_extreme_volatility_benchmark_data(size: usize) -> HyperliquidData {
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
            // Extreme volatility
            let volatility = (i as f64 * 0.2).sin() * price * 0.1; // 10% swings
            price += volatility;
            price.max(1000.0) // Prevent unrealistic prices
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: prices.iter().map(|p| p - 20.0).collect(),
        high: prices.iter().map(|p| p + 100.0).collect(),
        low: prices.iter().map(|p| p - 100.0).collect(),
        close: prices,
        volume: (0..size).map(|i| 50.0 + (i as f64 * 0.3).sin().abs() * 200.0).collect(),
        funding_rates: (0..size).map(|i| 0.0001 + (i as f64 * 0.1).sin() * 0.001).collect(),
        funding_timestamps: datetime,
    }
}

/// Create minimal benchmark data
fn create_minimal_benchmark_data(size: usize) -> HyperliquidData {
    let datetime: Vec<DateTime<FixedOffset>> = (0..size)
        .map(|i| {
            DateTime::from_timestamp(1640995200 + i as i64 * 3600, 0)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap())
        })
        .collect();

    HyperliquidData {
        ticker: "BTC".to_string(),
        datetime: datetime.clone(),
        open: vec![47000.0; size],
        high: vec![47010.0; size],
        low: vec![46990.0; size],
        close: vec![47005.0; size],
        volume: vec![100.0; size],
        funding_rates: vec![0.0001; size],
        funding_timestamps: datetime,
    }
}

criterion_group!(
    benches,
    bench_data_conversion,
    bench_funding_rate_lookup,
    bench_backtesting_workflow,
    bench_funding_calculations,
    bench_strategy_execution,
    bench_csv_export,
    bench_funding_report,
    bench_memory_operations,
    bench_concurrent_operations,
    bench_memory_allocation,
    bench_data_structure_ops,
    bench_api_response_processing,
    bench_strategy_types,
    bench_report_generation,
    bench_error_handling,
    bench_data_validation,
    bench_large_dataset_operations,
    bench_memory_intensive_concurrent,
    bench_api_simulation_realistic,
    bench_funding_arbitrage_detailed,
    bench_edge_cases
);

criterion_main!(benches);