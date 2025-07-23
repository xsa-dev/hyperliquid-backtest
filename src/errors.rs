//! Error types for Hyperliquid backtesting operations

use chrono::{DateTime, FixedOffset};
use thiserror::Error;

/// Result type alias for consistent error handling throughout the crate
pub type Result<T> = std::result::Result<T, HyperliquidBacktestError>;

/// Main error type for Hyperliquid backtesting operations
#[derive(Debug, Error)]
pub enum HyperliquidBacktestError {
    /// Error from Hyperliquid API operations
    #[error("Hyperliquid API error: {0}")]
    HyperliquidApi(String),
    
    /// Error during data conversion between formats
    #[error("Data conversion error: {0}")]
    DataConversion(String),
    
    /// Invalid time range specified
    #[error("Invalid time range: start {start} >= end {end}")]
    InvalidTimeRange { start: u64, end: u64 },
    
    /// Unsupported time interval
    #[error("Unsupported interval: {0}")]
    UnsupportedInterval(String),
    
    /// Missing funding data for a specific timestamp
    #[error("Missing funding data for timestamp: {0}")]
    MissingFundingData(DateTime<FixedOffset>),
    
    /// General backtesting error
    #[error("Backtesting error: {0}")]
    Backtesting(String),
    
    /// Network or HTTP related errors
    #[error("Network error: {0}")]
    Network(String),
    
    /// JSON parsing errors
    #[error("JSON parsing error: {0}")]
    JsonParsing(#[from] serde_json::Error),
    
    /// CSV processing errors
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    
    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    /// HTTP client errors
    #[error("HTTP client error: {0}")]
    Http(String),
    
    /// Date/time parsing errors
    #[error("DateTime parsing error: {0}")]
    DateTimeParsing(#[from] chrono::ParseError),
    
    /// Numeric parsing errors
    #[error("Number parsing error: {0}")]
    NumberParsing(String),
    
    /// Configuration errors
    #[error("Configuration error: {0}")]
    Configuration(String),
    
    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),
    
    /// Rate limiting errors
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    
    /// Authentication errors
    #[error("Authentication error: {0}")]
    Authentication(String),
    
    /// Data integrity errors
    #[error("Data integrity error: {0}")]
    DataIntegrity(String),
}

// Error conversion implementations for external library errors

impl From<std::num::ParseFloatError> for HyperliquidBacktestError {
    fn from(err: std::num::ParseFloatError) -> Self {
        HyperliquidBacktestError::NumberParsing(err.to_string())
    }
}

impl From<std::num::ParseIntError> for HyperliquidBacktestError {
    fn from(err: std::num::ParseIntError) -> Self {
        HyperliquidBacktestError::NumberParsing(err.to_string())
    }
}

// Helper methods for error creation and user guidance
impl HyperliquidBacktestError {
    /// Create a new API error with context
    pub fn api_error(message: impl Into<String>) -> Self {
        Self::HyperliquidApi(message.into())
    }
    
    /// Create a new data conversion error with context
    pub fn conversion_error(message: impl Into<String>) -> Self {
        Self::DataConversion(message.into())
    }
    
    /// Create a new validation error with context
    pub fn validation_error(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }
    
    /// Create a new configuration error with context
    pub fn config_error(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }
    
    /// Get user-friendly error message with suggestions for resolution
    pub fn user_message(&self) -> String {
        match self {
            Self::HyperliquidApi(msg) => {
                format!(
                    "Hyperliquid API Error: {}\n\n\
                    💡 Suggestions:\n\
                    • Check your internet connection\n\
                    • Verify the trading pair symbol (e.g., 'BTC', 'ETH')\n\
                    • Ensure the time range is valid and not too large\n\
                    • Try again in a few moments if rate limited",
                    msg
                )
            },
            Self::UnsupportedInterval(interval) => {
                format!(
                    "Unsupported time interval: '{}'\n\n\
                    💡 Supported intervals:\n\
                    • '1m' - 1 minute\n\
                    • '5m' - 5 minutes\n\
                    • '15m' - 15 minutes\n\
                    • '1h' - 1 hour\n\
                    • '4h' - 4 hours\n\
                    • '1d' - 1 day\n\n\
                    Example: HyperliquidData::fetch(\"BTC\", \"1h\", start, end)",
                    interval
                )
            },
            Self::InvalidTimeRange { start, end } => {
                format!(
                    "Invalid time range: start time ({}) must be before end time ({})\n\n\
                    💡 Suggestions:\n\
                    • Ensure start_time < end_time\n\
                    • Use Unix timestamps in seconds\n\
                    • Example: let start = Utc::now().timestamp() - 86400; // 24 hours ago",
                    start, end
                )
            },
            Self::MissingFundingData(timestamp) => {
                format!(
                    "Missing funding data for timestamp: {}\n\n\
                    💡 This usually means:\n\
                    • The timestamp is outside the funding data range\n\
                    • Funding data is not available for this time period\n\
                    • Consider using a different time range or disabling funding calculations",
                    timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                )
            },
            Self::DataConversion(msg) => {
                format!(
                    "Data conversion error: {}\n\n\
                    💡 This usually indicates:\n\
                    • Invalid data format from the API\n\
                    • Corrupted or incomplete data\n\
                    • Try fetching data for a different time period",
                    msg
                )
            },
            Self::Network(msg) => {
                format!(
                    "Network error: {}\n\n\
                    💡 Suggestions:\n\
                    • Check your internet connection\n\
                    • Verify firewall settings\n\
                    • Try again in a few moments\n\
                    • Consider using a VPN if in a restricted region",
                    msg
                )
            },
            Self::RateLimit(msg) => {
                format!(
                    "Rate limit exceeded: {}\n\n\
                    💡 Suggestions:\n\
                    • Wait a few minutes before making more requests\n\
                    • Reduce the frequency of API calls\n\
                    • Consider caching data locally\n\
                    • Use larger time intervals to fetch less data",
                    msg
                )
            },
            Self::Validation(msg) => {
                format!(
                    "Validation error: {}\n\n\
                    💡 Please check:\n\
                    • Input parameters are within valid ranges\n\
                    • Required fields are not empty\n\
                    • Data types match expected formats\n\
                    Details: {}",
                    msg, msg
                )
            },
            Self::Configuration(msg) => {
                format!(
                    "Configuration error: {}\n\n\
                    💡 Please verify:\n\
                    • All required configuration values are set\n\
                    • Configuration file format is correct\n\
                    • Environment variables are properly set\n\
                    Details: {}",
                    msg, msg
                )
            },
            _ => self.to_string(),
        }
    }
    
    /// Check if this error is recoverable (user can retry)
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::RateLimit(_) | Self::HyperliquidApi(_)
        )
    }
    
    /// Check if this error is due to user input
    pub fn is_user_error(&self) -> bool {
        matches!(
            self,
            Self::UnsupportedInterval(_) | 
            Self::InvalidTimeRange { .. } | 
            Self::Validation(_) | 
            Self::Configuration(_)
        )
    }
    
    /// Get error category for logging/monitoring
    pub fn category(&self) -> &'static str {
        match self {
            Self::HyperliquidApi(_) => "api",
            Self::DataConversion(_) => "data",
            Self::InvalidTimeRange { .. } => "validation",
            Self::UnsupportedInterval(_) => "validation",
            Self::MissingFundingData(_) => "data",
            Self::Backtesting(_) => "computation",
            Self::Network(_) => "network",
            Self::JsonParsing(_) => "parsing",
            Self::Csv(_) => "csv",
            Self::Io(_) => "io",
            Self::Http(_) => "network",
            Self::DateTimeParsing(_) => "parsing",
            Self::NumberParsing(_) => "parsing",
            Self::Configuration(_) => "config",
            Self::Validation(_) => "validation",
            Self::RateLimit(_) => "rate_limit",
            Self::Authentication(_) => "auth",
            Self::DataIntegrity(_) => "data",
        }
    }
    /// Create a new HyperliquidApi error
    pub fn hyperliquid_api<S: Into<String>>(msg: S) -> Self {
        HyperliquidBacktestError::HyperliquidApi(msg.into())
    }

    /// Create a new DataConversion error
    pub fn data_conversion<S: Into<String>>(msg: S) -> Self {
        HyperliquidBacktestError::DataConversion(msg.into())
    }

    /// Create a new InvalidTimeRange error
    pub fn invalid_time_range(start: u64, end: u64) -> Self {
        HyperliquidBacktestError::InvalidTimeRange { start, end }
    }

    /// Create a new UnsupportedInterval error
    pub fn unsupported_interval<S: Into<String>>(interval: S) -> Self {
        HyperliquidBacktestError::UnsupportedInterval(interval.into())
    }

    /// Create a new MissingFundingData error
    pub fn missing_funding_data(timestamp: DateTime<FixedOffset>) -> Self {
        HyperliquidBacktestError::MissingFundingData(timestamp)
    }

    /// Create a new Validation error
    pub fn validation<S: Into<String>>(msg: S) -> Self {
        HyperliquidBacktestError::Validation(msg.into())
    }

    /// Create a new Network error
    pub fn network<S: Into<String>>(msg: S) -> Self {
        HyperliquidBacktestError::Network(msg.into())
    }

    /// Create a new RateLimit error
    pub fn rate_limit<S: Into<String>>(msg: S) -> Self {
        HyperliquidBacktestError::RateLimit(msg.into())
    }
}

// Conversion for tokio join errors
impl From<tokio::task::JoinError> for HyperliquidBacktestError {
    fn from(err: tokio::task::JoinError) -> Self {
        HyperliquidBacktestError::Backtesting(format!("Task join error: {}", err))
    }
}

// Conversion for hyperliquid_rust_sdk errors
impl From<hyperliquid_rust_sdk::Error> for HyperliquidBacktestError {
    fn from(err: hyperliquid_rust_sdk::Error) -> Self {
        match err {
            hyperliquid_rust_sdk::Error::ClientRequest { status_code, error_code, error_message, error_data } => {
                HyperliquidBacktestError::HyperliquidApi(format!(
                    "Client error: status {}, code {:?}, message: {}, data: {:?}",
                    status_code, error_code, error_message, error_data
                ))
            },
            hyperliquid_rust_sdk::Error::ServerRequest { status_code, error_message } => {
                HyperliquidBacktestError::HyperliquidApi(format!(
                    "Server error: status {}, message: {}",
                    status_code, error_message
                ))
            },
            hyperliquid_rust_sdk::Error::GenericRequest(msg) => {
                HyperliquidBacktestError::Network(msg)
            },
            hyperliquid_rust_sdk::Error::JsonParse(_msg) => {
                HyperliquidBacktestError::JsonParsing(serde_json::from_str::<serde_json::Value>("").unwrap_err())
            },
            hyperliquid_rust_sdk::Error::Websocket(msg) => {
                HyperliquidBacktestError::Network(format!("WebSocket error: {}", msg))
            },
            _ => {
                HyperliquidBacktestError::HyperliquidApi(format!("Hyperliquid SDK error: {:?}", err))
            }
        }
    }
}

// Conversion for String errors (for test compatibility)
impl From<String> for HyperliquidBacktestError {
    fn from(msg: String) -> Self {
        HyperliquidBacktestError::Validation(msg)
    }
}
// Tests moved to tests/errors_tests.rs
