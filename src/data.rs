//! # Data Structures and Utilities for Hyperliquid Market Data
//!
//! This module provides the core data structures and fetching utilities for working with
//! Hyperliquid market data, including OHLC price data and funding rates for perpetual futures.
//!
//! ## Key Features
//!
//! - **Async Data Fetching**: Efficient retrieval of historical market data from Hyperliquid API
//! - **Funding Rate Integration**: Complete funding rate data for perpetual futures analysis
//! - **Multiple Time Intervals**: Support for 1m, 5m, 15m, 1h, 4h, and 1d intervals
//! - **Data Validation**: Comprehensive validation and error handling for data integrity
//! - **rs-backtester Compatibility**: Seamless conversion to rs-backtester Data format
//!
//! ## Usage Examples
//!
//! ### Basic Data Fetching
//!
//! ```rust,ignore
//! use hyperliquid_backtest::data::HyperliquidData;
//! use hyperliquid_backtest::errors::HyperliquidBacktestError;
//! use hyperliquid_backtest::prelude::*;
//! use chrono::Utc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     let end_time = Utc::now().timestamp() as u64;
//!     let start_time = end_time - (7 * 24 * 60 * 60); // 7 days ago
//!     
//!     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
//!     
//!     println!("Fetched {} data points for {}", data.len(), data.symbol);
//!     println!("Price range: ${:.2} - ${:.2}", data.price_range().0, data.price_range().1);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ### Working with Funding Rates
//!
//! ```rust,ignore
//! use hyperliquid_backtest::data::HyperliquidData;
//! use hyperliquid_backtest::errors::HyperliquidBacktestError;
//! use hyperliquid_backtest::prelude::*;
//! # let start_time = 0;
//! # let end_time = 0;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), HyperliquidBacktestError> {
//!     let data = HyperliquidData::fetch("ETH", "1h", start_time, end_time).await?;
//!     
//!     // Get funding statistics
//!     let funding_stats = data.funding_statistics()?;
//!     println!("Average funding rate: {:.4}%", funding_stats.average_rate * 100.0);
//!     println!("Funding volatility: {:.4}%", funding_stats.volatility * 100.0);
//!     
//!     // Get funding rate at specific time
//!     if let Some(rate) = data.get_funding_rate_at(data.datetime[100]) {
//!         println!("Funding rate at {}: {:.4}%", data.datetime[100], rate * 100.0);
//!     }
//!     
//!     Ok(())
//! }
//! ```

use crate::errors::{HyperliquidBacktestError, Result};
use chrono::{DateTime, FixedOffset, TimeZone};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main data structure for Hyperliquid market data
///
/// This structure contains OHLC price data along with funding rates for perpetual futures.
/// It provides a comprehensive view of market conditions including both price action and
/// funding dynamics.
///
/// ## Fields
///
/// - `symbol`: Trading pair symbol (e.g., "BTC", "ETH")
/// - `datetime`: Timestamps for each data point (UTC with timezone info)
/// - `open`, `high`, `low`, `close`: OHLC price data
/// - `volume`: Trading volume for each period
/// - `funding_rates`: Funding rates (NaN for non-funding periods)
///
/// ## Data Alignment
///
/// All arrays have the same length and are aligned by index. The funding_rates array
/// contains NaN values for periods where funding is not applied (typically every 8 hours).
///
/// ## Example
///
/// ```rust,ignore
/// use hyperliquid_backtest::data::HyperliquidData;
/// use hyperliquid_backtest::errors::HyperliquidBacktestError;
/// use hyperliquid_backtest::prelude::*;
/// # let start_time = 0;
/// # let end_time = 0;
///
/// #[tokio::main]
/// async fn main() -> Result<(), HyperliquidBacktestError> {
///     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
///     
///     // Access price data
///     println!("Latest close price: ${:.2}", data.close.last().unwrap());
///     
///     // Check data integrity
///     assert_eq!(data.datetime.len(), data.close.len());
///     assert_eq!(data.close.len(), data.funding_rates.len());
///     
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperliquidData {
    /// Trading pair symbol (e.g., "BTC", "ETH", "SOL")
    pub symbol: String,
    /// Array of timestamps for each data point (UTC with timezone information)
    pub datetime: Vec<DateTime<FixedOffset>>,
    /// Array of opening prices for each period
    pub open: Vec<f64>,
    /// Array of highest prices for each period
    pub high: Vec<f64>,
    /// Array of lowest prices for each period
    pub low: Vec<f64>,
    /// Array of closing prices for each period
    pub close: Vec<f64>,
    /// Array of trading volumes for each period
    pub volume: Vec<f64>,
    /// Array of funding rates (NaN for non-funding periods, typically every 8 hours)
    pub funding_rates: Vec<f64>,
}

/// Statistics about funding rates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundingStatistics {
    /// Average funding rate over the period
    pub average_rate: f64,
    /// Volatility (standard deviation) of funding rates
    pub volatility: f64,
    /// Minimum funding rate observed
    pub min_rate: f64,
    /// Maximum funding rate observed
    pub max_rate: f64,
    /// Number of positive funding periods
    pub positive_periods: usize,
    /// Number of negative funding periods
    pub negative_periods: usize,
    /// Total funding periods
    pub total_periods: usize,
}

/// Cacheable version of funding history for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheableFundingHistory {
    /// Trading pair symbol
    pub coin: String,
    /// Funding rate as string
    pub funding_rate: String,
    /// Premium as string
    pub premium: String,
    /// Timestamp
    pub time: u64,
}

impl From<&hyperliquid_rust_sdk::FundingHistoryResponse> for CacheableFundingHistory {
    fn from(response: &hyperliquid_rust_sdk::FundingHistoryResponse) -> Self {
        Self {
            coin: response.coin.clone(),
            funding_rate: response.funding_rate.clone(),
            premium: response.premium.clone(),
            time: response.time,
        }
    }
}

impl From<CacheableFundingHistory> for hyperliquid_rust_sdk::FundingHistoryResponse {
    fn from(cacheable: CacheableFundingHistory) -> Self {
        Self {
            coin: cacheable.coin,
            funding_rate: cacheable.funding_rate,
            premium: cacheable.premium,
            time: cacheable.time,
        }
    }
}

/// Data fetcher for Hyperliquid market data
pub struct HyperliquidDataFetcher {
    /// Hyperliquid info client
    info_client: hyperliquid_rust_sdk::InfoClient,
}

impl HyperliquidDataFetcher {
    /// Create a new HyperliquidDataFetcher
    pub async fn new() -> std::result::Result<Self, hyperliquid_rust_sdk::Error> {
        let info_client = hyperliquid_rust_sdk::InfoClient::new(None, Some(hyperliquid_rust_sdk::BaseUrl::Mainnet)).await?;
        
        Ok(Self {
            info_client,
        })
    }
    
    /// Get supported time intervals
    pub fn supported_intervals() -> &'static [&'static str] {
        &["1m", "5m", "15m", "1h", "4h", "1d"]
    }
    
    /// Check if a time interval is supported
    pub fn is_interval_supported(interval: &str) -> bool {
        Self::supported_intervals().contains(&interval)
    }
    
    /// Get maximum time range for a given interval
    pub fn max_time_range_for_interval(interval: &str) -> u64 {
        match interval {
            "1m" => 7 * 24 * 3600,      // 1 week for 1-minute data
            "5m" => 30 * 24 * 3600,     // 1 month for 5-minute data
            "15m" => 90 * 24 * 3600,    // 3 months for 15-minute data
            "1h" => 365 * 24 * 3600,    // 1 year for 1-hour data
            "4h" => 2 * 365 * 24 * 3600, // 2 years for 4-hour data
            "1d" => 5 * 365 * 24 * 3600, // 5 years for daily data
            _ => 365 * 24 * 3600,       // Default to 1 year
        }
    }
    
    /// Fetch OHLC data from Hyperliquid API
    pub async fn fetch_ohlc_data(
        &self,
        coin: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<hyperliquid_rust_sdk::CandlesSnapshotResponse>> {
        // Validate parameters
        HyperliquidData::validate_fetch_parameters(coin, interval, start_time, end_time)?;
        
        // Fetch data from API
        let candles = self.info_client
            .candles_snapshot(coin.to_string(), interval.to_string(), start_time, end_time)
            .await
            .map_err(|e| HyperliquidBacktestError::from(e))?;
        
        // Validate response
        self.validate_ohlc_response(&candles)?;
        
        Ok(candles)
    }
    
    /// Fetch funding history from Hyperliquid API
    pub async fn fetch_funding_history(
        &self,
        coin: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Vec<hyperliquid_rust_sdk::FundingHistoryResponse>> {
        // Validate parameters
        if coin.is_empty() {
            return Err(HyperliquidBacktestError::validation("Coin cannot be empty"));
        }
        
        if start_time >= end_time {
            return Err(HyperliquidBacktestError::invalid_time_range(start_time, end_time));
        }
        
        // Fetch data from API
        let funding_history = self.info_client
            .funding_history(coin.to_string(), start_time, Some(end_time))
            .await
            .map_err(|e| HyperliquidBacktestError::from(e))?;
        
        // Validate response
        self.validate_funding_response(&funding_history)?;
        
        Ok(funding_history)
    }
    
    /// Validate OHLC response
    fn validate_ohlc_response(&self, candles: &[hyperliquid_rust_sdk::CandlesSnapshotResponse]) -> Result<()> {
        if candles.is_empty() {
            return Err(HyperliquidBacktestError::validation("No OHLC data returned from API"));
        }

        // Validate each candle
        for (i, candle) in candles.iter().enumerate() {
            // Check that OHLC values can be parsed as floats
            candle.open.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid open price '{}' at index {}", candle.open, i)
                ))?;
            
            candle.high.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid high price '{}' at index {}", candle.high, i)
                ))?;
            
            candle.low.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid low price '{}' at index {}", candle.low, i)
                ))?;
            
            candle.close.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid close price '{}' at index {}", candle.close, i)
                ))?;
            
            candle.vlm.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid volume '{}' at index {}", candle.vlm, i)
                ))?;

            // Validate timestamp
            if candle.time_open >= candle.time_close {
                return Err(HyperliquidBacktestError::validation(
                    format!("Invalid candle timestamps: open {} >= close {} at index {}", 
                        candle.time_open, candle.time_close, i)
                ));
            }
        }

        // Check chronological order
        for i in 1..candles.len() {
            if candles[i].time_open <= candles[i - 1].time_open {
                return Err(HyperliquidBacktestError::validation(
                    format!("Candles not in chronological order at indices {} and {}", i - 1, i)
                ));
            }
        }

        Ok(())
    }
    
    /// Validate funding response
    fn validate_funding_response(&self, funding_history: &[hyperliquid_rust_sdk::FundingHistoryResponse]) -> Result<()> {
        if funding_history.is_empty() {
            return Ok(()); // Empty funding history is valid
        }

        // Validate each funding entry
        for (i, entry) in funding_history.iter().enumerate() {
            // Check that funding rate can be parsed as float
            entry.funding_rate.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid funding rate '{}' at index {}", entry.funding_rate, i)
                ))?;
            
            // Check that premium can be parsed as float
            entry.premium.parse::<f64>()
                .map_err(|_| HyperliquidBacktestError::data_conversion(
                    format!("Invalid premium '{}' at index {}", entry.premium, i)
                ))?;
        }

        // Check chronological order
        for i in 1..funding_history.len() {
            if funding_history[i].time <= funding_history[i - 1].time {
                return Err(HyperliquidBacktestError::validation(
                    format!("Funding history not in chronological order at indices {} and {}", i - 1, i)
                ));
            }
        }

        Ok(())
    }
    
    /// Align OHLC and funding data
    pub fn align_ohlc_and_funding_data(
        &self,
        ohlc_data: &[hyperliquid_rust_sdk::CandlesSnapshotResponse],
        funding_data: &[hyperliquid_rust_sdk::FundingHistoryResponse],
    ) -> Result<(Vec<DateTime<FixedOffset>>, Vec<f64>)> {
        if ohlc_data.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut aligned_timestamps = Vec::new();
        let mut aligned_funding_rates = Vec::new();

        // Convert funding data to a more searchable format
        let funding_map: HashMap<u64, f64> = funding_data
            .iter()
            .map(|entry| {
                let rate = entry.funding_rate.parse::<f64>()
                    .unwrap_or(0.0); // Default to 0 if parsing fails
                (entry.time, rate)
            })
            .collect();

        // For each OHLC timestamp, find the corresponding or nearest funding rate
        for candle in ohlc_data {
            let ohlc_timestamp = candle.time_open;
            let datetime = FixedOffset::east_opt(0)
                .ok_or_else(|| HyperliquidBacktestError::data_conversion(
                    "Failed to create UTC timezone offset".to_string()
                ))?
                .timestamp_opt(ohlc_timestamp as i64, 0)
                .single()
                .ok_or_else(|| HyperliquidBacktestError::data_conversion(
                    format!("Invalid timestamp {}", ohlc_timestamp)
                ))?;

            // Find the funding rate for this timestamp
            let funding_rate = self.find_funding_rate_for_timestamp(ohlc_timestamp, &funding_map);
            
            aligned_timestamps.push(datetime);
            aligned_funding_rates.push(funding_rate);
        }

        Ok((aligned_timestamps, aligned_funding_rates))
    }
    
    /// Find funding rate for a specific timestamp
    fn find_funding_rate_for_timestamp(
        &self,
        timestamp: u64,
        funding_map: &HashMap<u64, f64>,
    ) -> f64 {
        // First, try exact match
        if let Some(&rate) = funding_map.get(&timestamp) {
            return rate;
        }

        // If no exact match, find the closest funding rate before this timestamp
        let mut best_timestamp = 0;
        let mut best_rate = 0.0;

        for (&funding_timestamp, &rate) in funding_map.iter() {
            if funding_timestamp <= timestamp && funding_timestamp > best_timestamp {
                best_timestamp = funding_timestamp;
                best_rate = rate;
            }
        }

        // If no funding rate found before this timestamp, try to find one after
        if best_timestamp == 0 {
            let mut closest_timestamp = u64::MAX;
            for (&funding_timestamp, &rate) in funding_map.iter() {
                if funding_timestamp > timestamp && funding_timestamp < closest_timestamp {
                    closest_timestamp = funding_timestamp;
                    best_rate = rate;
                }
            }
        }

        best_rate
    }
}

impl HyperliquidDataFetcher {
    /// Create a new HyperliquidDataFetcher with custom error type
    pub async fn new_with_custom_error() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let info_client = hyperliquid_rust_sdk::InfoClient::new(None, Some(hyperliquid_rust_sdk::BaseUrl::Mainnet)).await?;
        Ok(Self { info_client })
    }
}

impl HyperliquidData {
    /// Fetch historical market data from Hyperliquid API
    ///
    /// This is the primary method for obtaining market data for backtesting. It fetches both
    /// OHLC price data and funding rate information from the Hyperliquid API.
    ///
    /// # Arguments
    ///
    /// * `coin` - Trading pair symbol (e.g., "BTC", "ETH", "SOL")
    /// * `interval` - Time interval for candles ("1m", "5m", "15m", "1h", "4h", "1d")
    /// * `start_time` - Start timestamp in Unix seconds
    /// * `end_time` - End timestamp in Unix seconds
    ///
    /// # Returns
    ///
    /// Returns a `Result<HyperliquidData, HyperliquidBacktestError>` containing the market data
    /// or an error if the fetch operation fails.
    ///
    /// # Errors
    ///
    /// This method can return several types of errors:
    /// - `UnsupportedInterval` - If the interval is not supported
    /// - `InvalidTimeRange` - If start_time >= end_time
    /// - `HyperliquidApi` - If the API request fails
    /// - `DataConversion` - If the response data is invalid
    /// - `Network` - If there are network connectivity issues
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use hyperliquid_backtest::data::HyperliquidData;
    /// use hyperliquid_backtest::errors::HyperliquidBacktestError;
    /// use chrono::Utc;
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<(), HyperliquidBacktestError> {
    ///     let end_time = Utc::now().timestamp() as u64;
    ///     let start_time = end_time - (24 * 60 * 60); // 24 hours ago
    ///     
    ///     // Fetch BTC data with 1-hour intervals
    ///     let data = HyperliquidData::fetch("BTC", "1h", start_time, end_time).await?;
    ///     
    ///     println!("Fetched {} data points", data.len());
    ///     println!("Latest price: ${:.2}", data.close.last().unwrap());
    ///     
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Performance Notes
    ///
    /// - Larger time ranges will take longer to fetch and process
    /// - Consider caching data locally for repeated backtests
    /// - Use appropriate intervals for your analysis (higher frequency = more data)
    pub async fn fetch(
        coin: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<Self> {
        let fetcher = HyperliquidDataFetcher::new().await
            .map_err(|e| HyperliquidBacktestError::HyperliquidApi(e.to_string()))?;
        
        // Fetch OHLC data
        let ohlc_data = fetcher.fetch_ohlc_data(coin, interval, start_time, end_time).await?;
        
        // Fetch funding data
        let funding_data = fetcher.fetch_funding_history(coin, start_time, end_time).await?;
        
        // Align and convert data
        let (aligned_timestamps, aligned_funding_rates) = 
            fetcher.align_ohlc_and_funding_data(&ohlc_data, &funding_data)?;
        
        // Convert OHLC data
        let mut open = Vec::new();
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();
        let mut volume = Vec::new();
        
        for candle in &ohlc_data {
            open.push(candle.open.parse::<f64>()?);
            high.push(candle.high.parse::<f64>()?);
            low.push(candle.low.parse::<f64>()?);
            close.push(candle.close.parse::<f64>()?);
            volume.push(candle.vlm.parse::<f64>()?);
        }
        
        let data = Self::with_ohlc_and_funding_data(
            coin.to_string(),
            aligned_timestamps,
            open,
            high,
            low,
            close,
            volume,
            aligned_funding_rates,
        )?;
        
        // Validate the final data
        data.validate_all_data()?;
        
        Ok(data)
    }

    /// Create a new HyperliquidData instance with OHLC data only
    ///
    /// This constructor creates a HyperliquidData instance with OHLC price data but no funding
    /// rate information. Funding rates will be set to NaN for all periods.
    ///
    /// # Arguments
    ///
    /// * `symbol` - Trading pair symbol
    /// * `datetime` - Vector of timestamps
    /// * `open` - Vector of opening prices
    /// * `high` - Vector of high prices
    /// * `low` - Vector of low prices
    /// * `close` - Vector of closing prices
    /// * `volume` - Vector of trading volumes
    ///
    /// # Returns
    ///
    /// Returns a `Result<HyperliquidData, HyperliquidBacktestError>` or a validation error
    /// if the input arrays have different lengths.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use hyperliquid_backtest::data::HyperliquidData;
    /// use hyperliquid_backtest::errors::HyperliquidBacktestError;
    /// use chrono::{DateTime, FixedOffset, Utc};
    ///
    /// let timestamps = vec![Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap())];
    /// let prices = vec![50000.0];
    /// let volumes = vec![1000.0];
    ///
    /// let data = HyperliquidData::with_ohlc_data(
    ///     "BTC".to_string(),
    ///     timestamps,
    ///     prices.clone(), // open
    ///     prices.clone(), // high
    ///     prices.clone(), // low
    ///     prices.clone(), // close
    ///     volumes,
    /// )?;
    /// # Ok::<(), HyperliquidBacktestError>(())
    /// ```
    pub fn with_ohlc_data(
        symbol: String,
        datetime: Vec<DateTime<FixedOffset>>,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
    ) -> Result<Self> {
        // Validate data arrays have the same length
        let len = datetime.len();
        if open.len() != len || high.len() != len || low.len() != len || close.len() != len || volume.len() != len {
            return Err(HyperliquidBacktestError::validation(
                "All data arrays must have the same length"
            ));
        }
        
        // Create instance with empty funding rates
        let funding_rates = vec![f64::NAN; len];
        
        Ok(Self {
            symbol,
            datetime,
            open,
            high,
            low,
            close,
            volume,
            funding_rates,
        })
    }
    
    /// Create a new HyperliquidData instance with OHLC and funding data
    pub fn with_ohlc_and_funding_data(
        symbol: String,
        datetime: Vec<DateTime<FixedOffset>>,
        open: Vec<f64>,
        high: Vec<f64>,
        low: Vec<f64>,
        close: Vec<f64>,
        volume: Vec<f64>,
        funding_rates: Vec<f64>,
    ) -> Result<Self> {
        // Validate data arrays have the same length
        let len = datetime.len();
        if open.len() != len || high.len() != len || low.len() != len || close.len() != len || 
           volume.len() != len || funding_rates.len() != len {
            return Err(HyperliquidBacktestError::validation(
                "All data arrays must have the same length"
            ));
        }
        
        Ok(Self {
            symbol,
            datetime,
            open,
            high,
            low,
            close,
            volume,
            funding_rates,
        })
    }
    
    /// Get the number of data points
    pub fn len(&self) -> usize {
        self.datetime.len()
    }
    
    /// Check if the data is empty
    pub fn is_empty(&self) -> bool {
        self.datetime.is_empty()
    }
    
    /// Validate all data for consistency
    pub fn validate_all_data(&self) -> Result<()> {
        // Check that all arrays have the same length
        let len = self.datetime.len();
        if self.open.len() != len || self.high.len() != len || self.low.len() != len || 
           self.close.len() != len || self.volume.len() != len || self.funding_rates.len() != len {
            return Err(HyperliquidBacktestError::validation(
                "All data arrays must have the same length"
            ));
        }
        
        // Check that high >= low for all candles
        for i in 0..len {
            if self.high[i] < self.low[i] {
                return Err(HyperliquidBacktestError::validation(
                    format!("High price {} is less than low price {} at index {}", 
                        self.high[i], self.low[i], i)
                ));
            }
        }
        
        // Check that timestamps are in chronological order
        for i in 1..len {
            if self.datetime[i] <= self.datetime[i - 1] {
                return Err(HyperliquidBacktestError::validation(
                    format!("Timestamps not in chronological order at indices {} and {}", 
                        i - 1, i)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Convert to rs-backtester Data format
    pub fn to_rs_backtester_data(&self) -> rs_backtester::datas::Data {
        // Create a new Data struct using the load method pattern
        // Since the fields might be private in the version we're using,
        // let's create a temporary CSV and load it
        use std::io::Write;
        use tempfile::NamedTempFile;
        
        // Create a temporary CSV file
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        writeln!(temp_file, "DATE,OPEN,HIGH,LOW,CLOSE").expect("Failed to write header");
        
        for i in 0..self.datetime.len() {
            writeln!(
                temp_file,
                "{},{},{},{},{}",
                self.datetime[i].to_rfc3339(),
                self.open[i],
                self.high[i],
                self.low[i],
                self.close[i]
            ).expect("Failed to write data");
        }
        
        temp_file.flush().expect("Failed to flush temp file");
        
        // Load the data using the rs-backtester load method
        rs_backtester::datas::Data::load(
            temp_file.path().to_str().unwrap(),
            &self.symbol
        ).expect("Failed to load data")
    }
    
    /// Get funding rate at a specific timestamp
    pub fn get_funding_rate_at(&self, timestamp: DateTime<FixedOffset>) -> Option<f64> {
        // Find the index of the timestamp
        if let Some(index) = self.datetime.iter().position(|&t| t == timestamp) {
            let rate = self.funding_rates[index];
            if !rate.is_nan() {
                return Some(rate);
            }
        }
        
        // If not found or NaN, return None
        None
    }
    
    /// Get the price (close) at or near a specific timestamp
    pub fn get_price_at_or_near(&self, timestamp: DateTime<FixedOffset>) -> Option<f64> {
        if self.datetime.is_empty() {
            return None;
        }

        // Find exact match first
        if let Some(index) = self.datetime.iter().position(|&t| t == timestamp) {
            return Some(self.close[index]);
        }

        // Find the closest timestamp
        let mut closest_index = 0;
        let mut min_diff = i64::MAX;

        for (i, &dt) in self.datetime.iter().enumerate() {
            let diff = (dt.timestamp() - timestamp.timestamp()).abs();
            if diff < min_diff {
                min_diff = diff;
                closest_index = i;
            }
        }

        // Return the price at the closest timestamp
        // Only return if within a reasonable time window (e.g., 24 hours)
        if min_diff <= 24 * 3600 {
            Some(self.close[closest_index])
        } else {
            None
        }
    }
    
    /// Calculate funding statistics
    pub fn calculate_funding_statistics(&self) -> FundingStatistics {
        let mut valid_rates = Vec::new();
        let mut positive_periods = 0;
        let mut negative_periods = 0;
        
        // Collect valid funding rates
        for &rate in &self.funding_rates {
            if !rate.is_nan() {
                valid_rates.push(rate);
                
                if rate > 0.0 {
                    positive_periods += 1;
                } else if rate < 0.0 {
                    negative_periods += 1;
                }
            }
        }
        
        // Calculate statistics
        let total_periods = valid_rates.len();
        let average_rate = if total_periods > 0 {
            valid_rates.iter().sum::<f64>() / total_periods as f64
        } else {
            0.0
        };
        
        let min_rate = valid_rates.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_rate = valid_rates.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
        
        // Calculate volatility (standard deviation)
        let volatility = if total_periods > 1 {
            let variance = valid_rates.iter()
                .map(|&r| (r - average_rate).powi(2))
                .sum::<f64>() / (total_periods - 1) as f64;
            variance.sqrt()
        } else {
            0.0
        };
        
        FundingStatistics {
            average_rate,
            volatility,
            min_rate: if min_rate.is_finite() { min_rate } else { 0.0 },
            max_rate: if max_rate.is_finite() { max_rate } else { 0.0 },
            positive_periods,
            negative_periods,
            total_periods,
        }
    }
    
    /// Validate fetch parameters
    pub fn validate_fetch_parameters(
        coin: &str,
        interval: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<()> {
        // Validate coin parameter
        if coin.is_empty() {
            return Err(HyperliquidBacktestError::validation("Coin cannot be empty"));
        }

        // Validate interval parameter
        if !HyperliquidDataFetcher::is_interval_supported(interval) {
            return Err(HyperliquidBacktestError::unsupported_interval(interval));
        }

        // Validate time range
        if start_time >= end_time {
            return Err(HyperliquidBacktestError::invalid_time_range(start_time, end_time));
        }

        // Validate that times are reasonable (not too far in the past or future)
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if start_time > current_time + 86400 { // Not more than 1 day in the future
            return Err(HyperliquidBacktestError::validation("Start time cannot be in the future"));
        }

        if end_time > current_time + 86400 { // Not more than 1 day in the future
            return Err(HyperliquidBacktestError::validation("End time cannot be in the future"));
        }

        // Validate that the time range is not too large (to prevent excessive API calls)
        let max_range_seconds = HyperliquidDataFetcher::max_time_range_for_interval(interval);

        if end_time - start_time > max_range_seconds {
            return Err(HyperliquidBacktestError::validation(
                format!("Time range too large for interval {}. Maximum range: {} days", 
                    interval, max_range_seconds / 86400)
            ));
        }

        Ok(())
    }
    
    /// Get list of popular trading pairs
    pub fn popular_trading_pairs() -> &'static [&'static str] {
        &["BTC", "ETH", "ATOM", "MATIC", "DYDX", "SOL", "AVAX", "BNB", "APE", "OP"]
    }
    
    /// Check if a trading pair is popular
    pub fn is_popular_pair(coin: &str) -> bool {
        Self::popular_trading_pairs().contains(&coin)
    }
}