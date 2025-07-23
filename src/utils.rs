//! Utility functions and helpers for Hyperliquid backtesting

use chrono::{DateTime, FixedOffset, TimeZone, Utc};
use crate::errors::{HyperliquidBacktestError, Result};

/// Time conversion utilities
pub mod time {
    use super::*;
    
    /// Convert Unix timestamp to DateTime<FixedOffset>
    pub fn unix_to_datetime(timestamp: u64) -> Result<DateTime<FixedOffset>> {
        let datetime = DateTime::from_timestamp(timestamp as i64, 0)
            .ok_or_else(|| HyperliquidBacktestError::DataConversion(
                format!("Invalid timestamp: {}", timestamp)
            ))?;
        
        Ok(datetime.with_timezone(&FixedOffset::east_opt(0).unwrap()))
    }
    
    /// Convert DateTime to Unix timestamp
    pub fn datetime_to_unix(dt: DateTime<FixedOffset>) -> u64 {
        dt.timestamp() as u64
    }
    
    /// Validate time range
    pub fn validate_time_range(start: u64, end: u64) -> Result<()> {
        if start >= end {
            return Err(HyperliquidBacktestError::InvalidTimeRange { start, end });
        }
        Ok(())
    }
    
    /// Get current Unix timestamp
    pub fn current_timestamp() -> u64 {
        Utc::now().timestamp() as u64
    }
}

/// Data validation utilities
pub mod validation {
    use super::*;
    
    /// Validate OHLC data consistency
    pub fn validate_ohlc_data(
        open: &[f64],
        high: &[f64],
        low: &[f64],
        close: &[f64],
    ) -> Result<()> {
        let len = open.len();
        if high.len() != len || low.len() != len || close.len() != len {
            return Err(HyperliquidBacktestError::DataConversion(
                "OHLC arrays have different lengths".to_string()
            ));
        }
        
        for i in 0..len {
            if high[i] < low[i] {
                return Err(HyperliquidBacktestError::DataConversion(
                    format!("High price {} is less than low price {} at index {}", 
                           high[i], low[i], i)
                ));
            }
            
            if open[i] < 0.0 || high[i] < 0.0 || low[i] < 0.0 || close[i] < 0.0 {
                return Err(HyperliquidBacktestError::DataConversion(
                    format!("Negative price found at index {}", i)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate funding rate data
    pub fn validate_funding_rates(rates: &[f64]) -> Result<()> {
        for (i, &rate) in rates.iter().enumerate() {
            if rate.is_nan() || rate.is_infinite() {
                return Err(HyperliquidBacktestError::DataConversion(
                    format!("Invalid funding rate at index {}: {}", i, rate)
                ));
            }
            
            // Reasonable bounds check for funding rates (typically between -1% and 1%)
            if rate.abs() > 0.01 {
                eprintln!("Warning: Unusually high funding rate at index {}: {:.4}%", 
                         i, rate * 100.0);
            }
        }
        
        Ok(())
    }
    
    /// Validate time interval string
    pub fn validate_interval(interval: &str) -> Result<()> {
        match interval {
            "1m" | "5m" | "15m" | "30m" | "1h" | "4h" | "1d" => Ok(()),
            _ => Err(HyperliquidBacktestError::UnsupportedInterval(interval.to_string())),
        }
    }
}

/// String conversion utilities
pub mod conversion {
    use super::*;
    
    /// Convert string to f64 with error handling
    pub fn string_to_f64(s: &str) -> Result<f64> {
        s.parse::<f64>()
            .map_err(|_| HyperliquidBacktestError::DataConversion(
                format!("Failed to parse '{}' as f64", s)
            ))
    }
    
    /// Convert string to u64 with error handling
    pub fn string_to_u64(s: &str) -> Result<u64> {
        s.parse::<u64>()
            .map_err(|_| HyperliquidBacktestError::DataConversion(
                format!("Failed to parse '{}' as u64", s)
            ))
    }
    
    /// Safely convert f64 to string with precision
    pub fn f64_to_string(value: f64, precision: usize) -> String {
        format!("{:.prec$}", value, prec = precision)
    }
}

/// Mathematical utilities
pub mod math {
    /// Calculate simple moving average
    pub fn sma(data: &[f64], period: usize) -> Vec<f64> {
        if period == 0 || data.len() < period {
            return Vec::new();
        }
        
        let mut result = Vec::with_capacity(data.len() - period + 1);
        
        for i in period - 1..data.len() {
            let sum: f64 = data[i - period + 1..=i].iter().sum();
            result.push(sum / period as f64);
        }
        
        result
    }
    
    /// Calculate exponential moving average
    pub fn ema(data: &[f64], period: usize) -> Vec<f64> {
        if data.is_empty() || period == 0 {
            return Vec::new();
        }
        
        let alpha = 2.0 / (period as f64 + 1.0);
        let mut result = Vec::with_capacity(data.len());
        
        // First value is just the first data point
        result.push(data[0]);
        
        for i in 1..data.len() {
            let ema_value = alpha * data[i] + (1.0 - alpha) * result[i - 1];
            result.push(ema_value);
        }
        
        result
    }
    
    /// Calculate standard deviation
    pub fn std_dev(data: &[f64]) -> f64 {
        if data.len() < 2 {
            return 0.0;
        }
        
        let mean = data.iter().sum::<f64>() / data.len() as f64;
        let variance = data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / (data.len() - 1) as f64;
        
        variance.sqrt()
    }
    
    /// Linear interpolation between two points
    pub fn lerp(x0: f64, y0: f64, x1: f64, y1: f64, x: f64) -> f64 {
        if (x1 - x0).abs() < f64::EPSILON {
            return y0;
        }
        
        y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    }
}

/// CSV utilities for data export
pub mod csv_utils {
    use super::*;
    use std::path::Path;
    
    /// Write data to CSV file
    pub fn write_csv<P: AsRef<Path>>(
        path: P,
        headers: &[&str],
        data: &[Vec<String>],
    ) -> Result<()> {
        let mut writer = csv::Writer::from_path(path)?;
        
        // Write headers
        writer.write_record(headers)?;
        
        // Write data rows
        for row in data {
            writer.write_record(row)?;
        }
        
        writer.flush()?;
        Ok(())
    }
    
    /// Read CSV file into vectors
    pub fn read_csv<P: AsRef<Path>>(path: P) -> Result<(Vec<String>, Vec<Vec<String>>)> {
        let mut reader = csv::Reader::from_path(path)?;
        
        // Get headers
        let headers = reader.headers()?.iter().map(|s| s.to_string()).collect();
        
        // Read data rows
        let mut data = Vec::new();
        for result in reader.records() {
            let record = result?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            data.push(row);
        }
        
        Ok((headers, data))
    }
}