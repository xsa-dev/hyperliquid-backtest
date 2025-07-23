use std::collections::HashMap;
use std::path::Path;
use std::fs::{self, File};
use std::io::{Read, Write};
use chrono::{DateTime, FixedOffset};
use serde::{Serialize, Deserialize};
use tracing::{info, warn, error};

use crate::unified_data_impl::{
    MarketData, OrderRequest, OrderResult, Signal, FundingPayment
};

/// Strategy parameter type for configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StrategyParam {
    /// String parameter
    String(String),
    /// Numeric parameter
    Number(f64),
    /// Boolean parameter
    Boolean(bool),
    /// Array of string parameters
    StringArray(Vec<String>),
    /// Array of numeric parameters
    NumberArray(Vec<f64>),
}

/// Strategy configuration for parameter management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyConfig {
    /// Strategy name
    pub name: String,
    
    /// Strategy description
    pub description: String,
    
    /// Strategy version
    pub version: String,
    
    /// Strategy parameters
    pub parameters: HashMap<String, StrategyParam>,
    
    /// Strategy metadata
    pub metadata: HashMap<String, String>,
}

impl StrategyConfig {
    /// Create a new strategy configuration
    pub fn new(name: &str, description: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
            version: version.to_string(),
            parameters: HashMap::new(),
            metadata: HashMap::new(),
        }
    }
    
    /// Add a string parameter
    pub fn with_string_param(mut self, key: &str, value: &str) -> Self {
        self.parameters.insert(key.to_string(), StrategyParam::String(value.to_string()));
        self
    }
    
    /// Add a numeric parameter
    pub fn with_number_param(mut self, key: &str, value: f64) -> Self {
        self.parameters.insert(key.to_string(), StrategyParam::Number(value));
        self
    }
    
    /// Add a boolean parameter
    pub fn with_bool_param(mut self, key: &str, value: bool) -> Self {
        self.parameters.insert(key.to_string(), StrategyParam::Boolean(value));
        self
    }
    
    /// Add a string array parameter
    pub fn with_string_array_param(mut self, key: &str, values: Vec<String>) -> Self {
        self.parameters.insert(key.to_string(), StrategyParam::StringArray(values));
        self
    }
    
    /// Add a numeric array parameter
    pub fn with_number_array_param(mut self, key: &str, values: Vec<f64>) -> Self {
        self.parameters.insert(key.to_string(), StrategyParam::NumberArray(values));
        self
    }
    
    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Get a string parameter
    pub fn get_string(&self, key: &str) -> Option<&String> {
        match self.parameters.get(key) {
            Some(StrategyParam::String(value)) => Some(value),
            _ => None,
        }
    }
    
    /// Get a numeric parameter
    pub fn get_number(&self, key: &str) -> Option<f64> {
        match self.parameters.get(key) {
            Some(StrategyParam::Number(value)) => Some(*value),
            _ => None,
        }
    }
    
    /// Get a boolean parameter
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        match self.parameters.get(key) {
            Some(StrategyParam::Boolean(value)) => Some(*value),
            _ => None,
        }
    }
    
    /// Get a string array parameter
    pub fn get_string_array(&self, key: &str) -> Option<&Vec<String>> {
        match self.parameters.get(key) {
            Some(StrategyParam::StringArray(value)) => Some(value),
            _ => None,
        }
    }
    
    /// Get a numeric array parameter
    pub fn get_number_array(&self, key: &str) -> Option<&Vec<f64>> {
        match self.parameters.get(key) {
            Some(StrategyParam::NumberArray(value)) => Some(value),
            _ => None,
        }
    }
    
    /// Save configuration to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize strategy config: {}", e))?;
        
        fs::write(path, json)
            .map_err(|e| format!("Failed to write strategy config: {}", e))?;
        
        Ok(())
    }
    
    /// Load configuration from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open strategy config file: {}", e))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read strategy config file: {}", e))?;
        
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse strategy config: {}", e))
    }
}

/// Strategy state for persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyState {
    /// Strategy name
    pub name: String,
    
    /// Strategy version
    pub version: String,
    
    /// Last update timestamp
    pub last_updated: DateTime<FixedOffset>,
    
    /// Current positions
    pub positions: HashMap<String, f64>,
    
    /// Current signals
    pub signals: HashMap<String, String>,
    
    /// Performance metrics
    pub metrics: HashMap<String, f64>,
    
    /// Custom state data
    pub custom_data: HashMap<String, serde_json::Value>,
}

impl StrategyState {
    /// Create a new strategy state
    pub fn new(name: &str, version: &str, timestamp: DateTime<FixedOffset>) -> Self {
        Self {
            name: name.to_string(),
            version: version.to_string(),
            last_updated: timestamp,
            positions: HashMap::new(),
            signals: HashMap::new(),
            metrics: HashMap::new(),
            custom_data: HashMap::new(),
        }
    }
    
    /// Update the timestamp
    pub fn update_timestamp(&mut self, timestamp: DateTime<FixedOffset>) {
        self.last_updated = timestamp;
    }
    
    /// Add or update a position
    pub fn set_position(&mut self, symbol: &str, size: f64) {
        self.positions.insert(symbol.to_string(), size);
    }
    
    /// Remove a position
    pub fn remove_position(&mut self, symbol: &str) {
        self.positions.remove(symbol);
    }
    
    /// Add or update a signal
    pub fn set_signal(&mut self, symbol: &str, signal: &str) {
        self.signals.insert(symbol.to_string(), signal.to_string());
    }
    
    /// Remove a signal
    pub fn remove_signal(&mut self, symbol: &str) {
        self.signals.remove(symbol);
    }
    
    /// Add or update a metric
    pub fn set_metric(&mut self, key: &str, value: f64) {
        self.metrics.insert(key.to_string(), value);
    }
    
    /// Add or update custom data
    pub fn set_custom_data<T: Serialize>(&mut self, key: &str, value: &T) -> Result<(), String> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| format!("Failed to serialize custom data: {}", e))?;
        
        self.custom_data.insert(key.to_string(), json_value);
        Ok(())
    }
    
    /// Get custom data
    pub fn get_custom_data<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, String> {
        match self.custom_data.get(key) {
            Some(value) => {
                let deserialized = serde_json::from_value(value.clone())
                    .map_err(|e| format!("Failed to deserialize custom data: {}", e))?;
                Ok(Some(deserialized))
            },
            None => Ok(None),
        }
    }
    
    /// Save state to a file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize strategy state: {}", e))?;
        
        fs::write(path, json)
            .map_err(|e| format!("Failed to write strategy state: {}", e))?;
        
        Ok(())
    }
    
    /// Load state from a file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let mut file = File::open(path)
            .map_err(|e| format!("Failed to open strategy state file: {}", e))?;
        
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| format!("Failed to read strategy state file: {}", e))?;
        
        serde_json::from_str(&contents)
            .map_err(|e| format!("Failed to parse strategy state: {}", e))
    }
}

/// Trading strategy trait for unified strategy execution across all modes
pub trait TradingStrategy: Send + Sync {
    /// Get the strategy name
    fn name(&self) -> &str;
    
    /// Get the strategy configuration
    fn config(&self) -> &StrategyConfig;
    
    /// Get a mutable reference to the strategy configuration
    fn config_mut(&mut self) -> &mut StrategyConfig;
    
    /// Update the strategy configuration
    fn update_config(&mut self, config: StrategyConfig) -> Result<(), String>;
    
    /// Get the strategy state
    fn state(&self) -> &StrategyState;
    
    /// Get a mutable reference to the strategy state
    fn state_mut(&mut self) -> &mut StrategyState;
    
    /// Process market data and generate signals
    fn on_market_data(&mut self, data: &MarketData) -> Result<Vec<OrderRequest>, String>;
    
    /// Process order fill events
    fn on_order_fill(&mut self, fill: &OrderResult) -> Result<(), String>;
    
    /// Process funding payment events
    fn on_funding_payment(&mut self, payment: &FundingPayment) -> Result<(), String>;
    
    /// Get current strategy signals
    fn get_current_signals(&self) -> HashMap<String, Signal>;
    
    /// Initialize the strategy
    fn initialize(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// Shutdown the strategy
    fn shutdown(&mut self) -> Result<(), String> {
        Ok(())
    }
    
    /// Save strategy state to a file
    fn save_state<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        self.state().save_to_file(path)
    }
    
    /// Load strategy state from a file
    fn load_state<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let state = StrategyState::load_from_file(path)?;
        
        // Validate that the state belongs to this strategy
        if state.name != self.name() {
            return Err(format!(
                "Strategy name mismatch: expected {}, found {}",
                self.name(),
                state.name
            ));
        }
        
        // Update the strategy state
        *self.state_mut() = state;
        
        Ok(())
    }
    
    /// Save strategy configuration to a file
    fn save_config<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        self.config().save_to_file(path)
    }
    
    /// Load strategy configuration from a file
    fn load_config<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        let config = StrategyConfig::load_from_file(path)?;
        
        // Validate that the config belongs to this strategy
        if config.name != self.name() {
            return Err(format!(
                "Strategy name mismatch: expected {}, found {}",
                self.name(),
                config.name
            ));
        }
        
        // Update the strategy configuration
        self.update_config(config)
    }
    
    /// Handle strategy-specific events
    fn on_event(&mut self, event_type: &str, data: &serde_json::Value) -> Result<(), String> {
        // Default implementation does nothing
        Ok(())
    }
}

/// Base implementation for trading strategies
pub struct BaseTradingStrategy {
    /// Strategy configuration
    config: StrategyConfig,
    
    /// Strategy state
    state: StrategyState,
}

impl BaseTradingStrategy {
    /// Create a new base trading strategy
    pub fn new(name: &str, description: &str, version: &str) -> Self {
        let config = StrategyConfig::new(name, description, version);
        let state = StrategyState::new(
            name,
            version,
            chrono::Utc::now().with_timezone(&chrono::FixedOffset::east(0)),
        );
        
        Self { config, state }
    }
}

impl TradingStrategy for BaseTradingStrategy {
    fn name(&self) -> &str {
        &self.config.name
    }
    
    fn config(&self) -> &StrategyConfig {
        &self.config
    }
    
    fn config_mut(&mut self) -> &mut StrategyConfig {
        &mut self.config
    }
    
    fn update_config(&mut self, config: StrategyConfig) -> Result<(), String> {
        if config.name != self.config.name {
            return Err(format!(
                "Strategy name mismatch: expected {}, found {}",
                self.config.name,
                config.name
            ));
        }
        
        self.config = config;
        Ok(())
    }
    
    fn state(&self) -> &StrategyState {
        &self.state
    }
    
    fn state_mut(&mut self) -> &mut StrategyState {
        &mut self.state
    }
    
    fn on_market_data(&mut self, _data: &MarketData) -> Result<Vec<OrderRequest>, String> {
        // Base implementation does nothing
        Ok(Vec::new())
    }
    
    fn on_order_fill(&mut self, _fill: &OrderResult) -> Result<(), String> {
        // Base implementation does nothing
        Ok(())
    }
    
    fn on_funding_payment(&mut self, _payment: &FundingPayment) -> Result<(), String> {
        // Base implementation does nothing
        Ok(())
    }
    
    fn get_current_signals(&self) -> HashMap<String, Signal> {
        // Base implementation returns empty signals
        HashMap::new()
    }
}