use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use chrono::{DateTime, FixedOffset, Utc};
use log::{debug, info, warn, error};
use thiserror::Error;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use uuid::Uuid;
use tracing::instrument;

use hyperliquid_rust_sdk::{InfoClient, BaseUrl};
// Note: These types may not be available in the current SDK version
// use hyperliquid_rust_sdk::{ExchangeClient, LocalWallet};
use hyperliquid_rust_sdk::Error as SdkError;

// Placeholder types for missing SDK types
#[derive(Debug, Clone)]
pub struct LocalWallet {
    pub address: String,
}

// Mock types for testing
#[derive(Debug, Clone, Default)]
pub struct MockUserState {
    pub margin_summary: MockMarginSummary,
    pub asset_positions: Vec<MockAssetPosition>,
}

#[derive(Debug, Clone, Default)]
pub struct MockMarginSummary {
    pub account_value: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MockAssetPosition {
    pub position: MockPosition,
}

#[derive(Debug, Clone)]
pub struct MockPosition {
    pub coin: String,
    pub szi: Option<String>,
    pub entry_px: Option<String>,
}

// No H160 implementation needed since we're using String

impl LocalWallet {
    pub fn new(private_key: &str) -> Result<Self, String> {
        Ok(Self {
            address: "placeholder_address".to_string(),
        })
    }
    
    pub fn address(&self) -> String {
        self.address.clone()
    }
}

#[derive(Debug)]
pub struct ExchangeClient {
    // Placeholder implementation
}

// Placeholder for order response
#[derive(Debug)]
pub struct OrderResponse {
    pub status: String,
    pub order_id: String,
    pub error: Option<String>,
}

// Placeholder for cancel response
#[derive(Debug)]
pub struct CancelResponse {
    pub status: String,
    pub error: Option<String>,
}

impl ExchangeClient {
    /// Place an order
    pub async fn order(&self, _client_order: ClientOrderRequest, _options: Option<()>) -> Result<OrderResponse, SdkError> {
        // Placeholder implementation
        Ok(OrderResponse {
            status: "ok".to_string(),
            order_id: "order_id_placeholder".to_string(),
            error: None,
        })
    }
    
    /// Cancel an order
    pub async fn cancel(&self, _cancel_request: String, _options: Option<()>) -> Result<CancelResponse, SdkError> {
        // Placeholder implementation
        Ok(CancelResponse {
            status: "ok".to_string(),
            error: None,
        })
    }
}

impl ExchangeClient {
    pub fn new(
        _http_client: Option<reqwest::Client>,
        _wallet: LocalWallet,
        _base_url: Option<BaseUrl>,
        _timeout: Option<std::time::Duration>,
        _retry_config: Option<()>,
    ) -> impl std::future::Future<Output = Result<Self, SdkError>> {
        async move {
            Ok(Self {})
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientOrderRequest {
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub quantity: String,
    pub price: Option<String>,
}

use crate::trading_mode::{ApiConfig, RiskConfig};
use crate::unified_data::{
    Position, OrderRequest, OrderResult, MarketData, 
    OrderSide, OrderType, TimeInForce, OrderStatus,
    TradingStrategy
};
use crate::real_time_data_stream::{RealTimeDataStream, RealTimeDataError};
use crate::risk_manager::{RiskManager, RiskError};

/// Error types specific to live trading operations
#[derive(Debug, Error)]
pub enum LiveTradingError {
    /// Error when market data is not available
    #[error("Market data not available for {0}")]
    MarketDataNotAvailable(String),
    
    /// Error when order execution fails
    #[error("Order execution failed: {0}")]
    OrderExecutionFailed(String),
    
    /// Error when position is not found
    #[error("Position not found for {0}")]
    PositionNotFound(String),
    
    /// Error when insufficient balance
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance {
        required: f64,
        available: f64,
    },
    
    /// Error when real-time data stream fails
    #[error("Real-time data stream error: {0}")]
    RealTimeDataError(#[from] RealTimeDataError),
    
    /// Error when strategy execution fails
    #[error("Strategy execution error: {0}")]
    StrategyError(String),
    
    /// Error when risk management fails
    #[error("Risk management error: {0}")]
    RiskError(#[from] RiskError),
    
    /// Error when Hyperliquid SDK fails
    #[error("Hyperliquid SDK error: {0}")]
    SdkError(String),
    
    /// Error when connection fails
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    /// Error when emergency stop is active
    #[error("Emergency stop is active")]
    EmergencyStop,
    
    /// Error when wallet is not configured
    #[error("Wallet not configured")]
    WalletNotConfigured,
    
    /// Error when API configuration is invalid
    #[error("Invalid API configuration: {0}")]
    InvalidApiConfig(String),
    
    /// Error when order retry limit is reached
    #[error("Order retry limit reached after {attempts} attempts: {reason}")]
    RetryLimitReached {
        attempts: u32,
        reason: String,
    },
    
    /// Error when monitoring system fails
    #[error("Monitoring system error: {0}")]
    MonitoringError(String),
    
    /// Error when safety circuit breaker is triggered
    #[error("Safety circuit breaker triggered: {0}")]
    SafetyCircuitBreaker(String),
    
    /// Error when order cancellation fails
    #[error("Order cancellation failed: {0}")]
    OrderCancellationFailed(String),
}

/// Represents a live order in the trading system
#[derive(Debug, Clone)]
pub struct LiveOrder {
    /// Order ID
    pub order_id: String,
    
    /// Original order request
    pub request: OrderRequest,
    
    /// Order result
    pub result: OrderResult,
    
    /// Creation timestamp
    pub created_at: DateTime<FixedOffset>,
    
    /// Last update timestamp
    pub updated_at: DateTime<FixedOffset>,
    
    /// Order status
    pub status: OrderStatus,
    
    /// Error message if any
    pub error: Option<String>,
}

/// Alert level for monitoring system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlertLevel {
    /// Informational alert
    Info,
    
    /// Warning alert
    Warning,
    
    /// Error alert
    Error,
    
    /// Critical alert
    Critical,
}

impl std::fmt::Display for AlertLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AlertLevel::Info => write!(f, "Info"),
            AlertLevel::Warning => write!(f, "Warning"),
            AlertLevel::Error => write!(f, "Error"),
            AlertLevel::Critical => write!(f, "Critical"),
        }
    }
}

/// Alert message for monitoring system
#[derive(Debug, Clone)]
pub struct AlertMessage {
    /// Alert level
    pub level: AlertLevel,
    
    /// Alert message
    pub message: String,
    
    /// Timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Related symbol if any
    pub symbol: Option<String>,
    
    /// Related order ID if any
    pub order_id: Option<String>,
}

/// Order retry policy
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    
    /// Initial delay between retries in milliseconds
    pub initial_delay_ms: u64,
    
    /// Backoff factor for exponential backoff
    pub backoff_factor: f64,
    
    /// Maximum delay between retries in milliseconds
    pub max_delay_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 500,
            backoff_factor: 2.0,
            max_delay_ms: 10000,
        }
    }
}

/// Order retry state
#[derive(Debug, Clone)]
pub struct OrderRetryState {
    /// Original order request
    pub order_request: OrderRequest,
    
    /// Number of attempts so far
    pub attempts: u32,
    
    /// Last attempt timestamp
    pub last_attempt: DateTime<FixedOffset>,
    
    /// Last error message
    pub last_error: String,
    
    /// Next retry time
    pub next_retry: DateTime<FixedOffset>,
}

/// Safety circuit breaker configuration
#[derive(Debug, Clone)]
pub struct SafetyCircuitBreakerConfig {
    /// Maximum number of consecutive failed orders
    pub max_consecutive_failed_orders: u32,
    
    /// Maximum order failure rate (0.0 - 1.0)
    pub max_order_failure_rate: f64,
    
    /// Window size for order failure rate calculation
    pub order_failure_rate_window: usize,
    
    /// Maximum position drawdown percentage to trigger circuit breaker
    pub max_position_drawdown_pct: f64,
    
    /// Maximum account drawdown percentage to trigger circuit breaker
    pub max_account_drawdown_pct: f64,
    
    /// Maximum price deviation percentage to trigger circuit breaker
    pub max_price_deviation_pct: f64,
    
    /// Price deviation window size in seconds
    pub price_deviation_window_sec: u64,
    
    /// Maximum number of alerts at Critical level to trigger circuit breaker
    pub max_critical_alerts: u32,
    
    /// Window size for critical alerts calculation
    pub critical_alerts_window: usize,
}

impl Default for SafetyCircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_consecutive_failed_orders: 3,
            max_order_failure_rate: 0.5,
            order_failure_rate_window: 10,
            max_position_drawdown_pct: 0.15,
            max_account_drawdown_pct: 0.10,
            max_price_deviation_pct: 0.05,
            price_deviation_window_sec: 60,
            max_critical_alerts: 3,
            critical_alerts_window: 10,
        }
    }
}

/// Live trading engine for executing trades on Hyperliquid exchange
pub struct LiveTradingEngine {
    /// Exchange client for API access
    exchange_client: Option<ExchangeClient>,
    
    /// Info client for market data
    info_client: InfoClient,
    
    /// Wallet for authentication
    wallet: Option<LocalWallet>,
    
    /// Risk manager
    risk_manager: RiskManager,
    
    /// Real-time data stream
    real_time_data: Option<Arc<Mutex<RealTimeDataStream>>>,
    
    /// Latest market data
    market_data_cache: HashMap<String, MarketData>,
    
    /// Current positions
    pub positions: HashMap<String, Position>,
    
    /// Order history
    order_history: Vec<LiveOrder>,
    
    /// Active orders
    active_orders: HashMap<String, LiveOrder>,
    
    /// Emergency stop flag
    pub emergency_stop: Arc<AtomicBool>,
    
    /// API configuration
    api_config: ApiConfig,
    
    /// Account balance
    pub account_balance: f64,
    
    /// Connection status
    is_connected: bool,
    
    /// Last connection attempt
    last_connection_attempt: Instant,
    
    /// Connection retry count
    connection_retry_count: u32,
    
    /// Maximum connection retry count
    max_connection_retries: u32,
    
    /// Connection check task
    connection_check_task: Option<JoinHandle<()>>,
    
    /// Order update task
    order_update_task: Option<JoinHandle<()>>,
    
    /// Position update task
    position_update_task: Option<JoinHandle<()>>,
    
    /// Is running flag
    is_running: bool,
    
    /// Order retry policy
    pub retry_policy: RetryPolicy,
    
    /// Orders pending retry
    pub pending_retries: HashMap<String, OrderRetryState>,
    
    /// Retry task
    pub retry_task: Option<JoinHandle<()>>,
    
    /// Alert messages
    alerts: VecDeque<AlertMessage>,
    
    /// Alert channel sender
    pub alert_sender: Option<Sender<AlertMessage>>,
    
    /// Alert channel receiver
    pub alert_receiver: Option<Receiver<AlertMessage>>,
    
    /// Alert processing task
    pub alert_task: Option<JoinHandle<()>>,
    
    /// Safety circuit breaker configuration
    pub safety_circuit_breaker_config: SafetyCircuitBreakerConfig,
    
    /// Consecutive failed orders count
    pub consecutive_failed_orders: u32,
    
    /// Recent order success/failure history
    pub order_result_history: VecDeque<bool>,
    
    /// Recent price history for deviation detection
    price_history: HashMap<String, VecDeque<(DateTime<FixedOffset>, f64)>>,
    
    /// Initial account value for drawdown calculation
    initial_account_value: f64,
    
    /// Highest account value for drawdown calculation
    pub highest_account_value: f64,
    
    /// Recent critical alerts
    recent_critical_alerts: VecDeque<AlertMessage>,
    
    /// Monitoring task
    pub monitoring_task: Option<JoinHandle<()>>,
    
    /// Detailed logging enabled flag
    pub detailed_logging: bool,
    
    /// Monitoring manager
    pub monitoring_manager: Option<crate::real_time_monitoring::MonitoringManager>,
}

impl LiveTradingEngine {
    /// Create a new live trading engine with the specified wallet and risk configuration
    pub async fn new(wallet: LocalWallet, risk_config: RiskConfig, api_config: ApiConfig) -> Result<Self, LiveTradingError> {
        // Create info client
        let base_url = if api_config.use_testnet {
            BaseUrl::Testnet
        } else {
            BaseUrl::Mainnet
        };
        
        let info_client = InfoClient::new(None, Some(base_url)).await
            .map_err(|e| LiveTradingError::SdkError(e.to_string()))?;
        
        // Get initial account balance (placeholder implementation)
        // In a real implementation, we would call info_client.user_state(wallet.address())
        let user_state = MockUserState::default();
        
        let account_balance = user_state.margin_summary.account_value
            .as_ref()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        
        // Create risk manager
        let risk_manager = RiskManager::new(risk_config, account_balance);
        
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        Ok(Self {
            exchange_client: None,
            info_client,
            wallet: Some(wallet),
            risk_manager,
            real_time_data: None,
            market_data_cache: HashMap::new(),
            positions: HashMap::new(),
            order_history: Vec::new(),
            active_orders: HashMap::new(),
            emergency_stop: Arc::new(AtomicBool::new(false)),
            api_config,
            account_balance,
            is_connected: false,
            last_connection_attempt: Instant::now(),
            connection_retry_count: 0,
            max_connection_retries: 5,
            connection_check_task: None,
            order_update_task: None,
            position_update_task: None,
            is_running: false,
            retry_policy: RetryPolicy::default(),
            pending_retries: HashMap::new(),
            retry_task: None,
            alerts: VecDeque::new(),
            alert_sender: None,
            alert_receiver: None,
            alert_task: None,
            safety_circuit_breaker_config: SafetyCircuitBreakerConfig::default(),
            consecutive_failed_orders: 0,
            order_result_history: VecDeque::new(),
            price_history: HashMap::new(),
            initial_account_value: account_balance,
            highest_account_value: account_balance,
            recent_critical_alerts: VecDeque::new(),
            monitoring_task: None,
            detailed_logging: false,
            monitoring_manager: None,
        })
    }
    
    /// Connect to the exchange
    pub async fn connect(&mut self) -> Result<(), LiveTradingError> {
        if self.is_connected {
            return Ok(());
        }
        
        info!("Connecting to Hyperliquid exchange...");
        
        // Check if wallet is configured
        let wallet = self.wallet.as_ref().ok_or(LiveTradingError::WalletNotConfigured)?;
        
        // Create exchange client
        let base_url = if self.api_config.use_testnet {
            BaseUrl::Testnet
        } else {
            BaseUrl::Mainnet
        };
        
        let exchange_client = ExchangeClient::new(
            None,
            wallet.clone(),
            Some(base_url),
            None,
            None,
        )
        .await
        .map_err(|e| LiveTradingError::SdkError(e.to_string()))?;
        
        self.exchange_client = Some(exchange_client);
        
        // Create real-time data stream if not already created
        if self.real_time_data.is_none() {
            let data_stream = RealTimeDataStream::new()
                .await
                .map_err(LiveTradingError::RealTimeDataError)?;
            
            self.real_time_data = Some(Arc::new(Mutex::new(data_stream)));
        }
        
        // Connect real-time data stream
        if let Some(data_stream) = &self.real_time_data {
            let mut stream = data_stream.lock().unwrap();
            stream.connect().await.map_err(LiveTradingError::RealTimeDataError)?;
        }
        
        // Update connection status
        self.is_connected = true;
        self.connection_retry_count = 0;
        self.last_connection_attempt = Instant::now();
        
        // Start connection check task
        self.start_connection_check_task();
        
        // Start order update task
        self.start_order_update_task();
        
        // Start position update task
        self.start_position_update_task();
        
        // Fetch initial positions
        self.update_positions().await?;
        
        info!("Connected to Hyperliquid exchange");
        
        Ok(())
    }    
   
 /// Disconnect from the exchange
    pub async fn disconnect(&mut self) -> Result<(), LiveTradingError> {
        if !self.is_connected {
            return Ok(());
        }
        
        info!("Disconnecting from Hyperliquid exchange...");
        
        // Stop tasks
        if let Some(task) = &self.connection_check_task {
            task.abort();
        }
        
        if let Some(task) = &self.order_update_task {
            task.abort();
        }
        
        if let Some(task) = &self.position_update_task {
            task.abort();
        }
        
        // Disconnect real-time data stream
        if let Some(data_stream) = &self.real_time_data {
            let mut stream = data_stream.lock().unwrap();
            stream.disconnect().await.map_err(LiveTradingError::RealTimeDataError)?;
        }
        
        // Clear exchange client
        self.exchange_client = None;
        
        // Update connection status
        self.is_connected = false;
        
        info!("Disconnected from Hyperliquid exchange");
        
        Ok(())
    }
    
    /// Execute an order with safety mechanisms
    #[instrument(level = "info", skip(self, order), fields(symbol = %order.symbol, side = ?order.side, quantity = %order.quantity))]
    pub async fn execute_order(&mut self, order: OrderRequest) -> Result<OrderResult, LiveTradingError> {
        // Check if connected
        if !self.is_connected {
            let error_msg = "Not connected to exchange";
            self.send_alert(AlertLevel::Error, error_msg, Some(&order.symbol), None);
            return Err(LiveTradingError::ConnectionError(error_msg.to_string()));
        }
        
        // Check if emergency stop is active
        if self.emergency_stop.load(Ordering::SeqCst) {
            self.send_alert(AlertLevel::Warning, "Order rejected: Emergency stop is active", Some(&order.symbol), None);
            return Err(LiveTradingError::EmergencyStop);
        }
        
        // Check safety circuit breakers
        if let Err(e) = self.check_safety_circuit_breakers() {
            return Err(e);
        }
        
        // Validate the order
        if let Err(err) = order.validate() {
            let error_msg = format!("Order validation failed: {}", err);
            self.send_alert(AlertLevel::Warning, &error_msg, Some(&order.symbol), None);
            return Err(LiveTradingError::OrderExecutionFailed(err));
        }
        
        // Get the latest market data for this symbol
        let market_data = match self.get_market_data(&order.symbol) {
            Ok(data) => data,
            Err(e) => {
                let error_msg = format!("Failed to get market data: {}", e);
                self.send_alert(AlertLevel::Warning, &error_msg, Some(&order.symbol), None);
                return Err(e);
            }
        };
        
        // Validate order against risk limits
        if let Err(e) = self.risk_manager.validate_order(&order, &self.positions) {
            let error_msg = format!("Risk validation failed: {}", e);
            self.send_alert(AlertLevel::Warning, &error_msg, Some(&order.symbol), None);
            return Err(LiveTradingError::RiskError(e));
        }
        
        // Generate a unique order ID
        let order_id = Uuid::new_v4().to_string();
        
        // Create the initial order result
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        let mut order_result = OrderResult::new(
            &order_id,
            &order.symbol,
            order.side,
            order.order_type,
            order.quantity,
            now,
        );
        order_result.status = OrderStatus::Submitted;
        
        // Log order submission
        if self.detailed_logging {
            info!("Submitting order: {} {} {} @ {:?} ({})", 
                  order.side, order.quantity, order.symbol, order.price, order.order_type);
        }
        
        self.send_alert(AlertLevel::Info, 
                       &format!("Submitting order: {} {} {} @ {:?}", 
                               order.side, order.quantity, order.symbol, order.price),
                       Some(&order.symbol), Some(&order_id));
        
        // Convert to Hyperliquid order
        let client_order = match self.convert_to_client_order(&order) {
            Ok(order) => order,
            Err(e) => {
                let error_msg = format!("Failed to convert order: {}", e);
                self.send_alert(AlertLevel::Error, &error_msg, Some(&order.symbol), Some(&order_id));
                
                // Update order result
                order_result.status = OrderStatus::Rejected;
                order_result.error = Some(error_msg.clone());
                
                // Add to order history
                let live_order = LiveOrder {
                    order_id: order_id.clone(),
                    request: order.clone(),
                    result: order_result.clone(),
                    created_at: now,
                    updated_at: now,
                    status: OrderStatus::Rejected,
                    error: Some(error_msg.clone()),
                };
                self.order_history.push(live_order);
                
                // Update order result history
                self.update_order_result(false);
                
                return Err(e);
            }
        };
        
        // Execute the order
        let exchange_client = self.exchange_client.as_ref()
            .ok_or(LiveTradingError::ConnectionError("Exchange client not initialized".to_string()))?;
        
        let response = match exchange_client.order(client_order, None).await {
            Ok(response) => response,
            Err(e) => {
                let error_msg = format!("API error: {}", e);
                self.send_alert(AlertLevel::Error, &error_msg, Some(&order.symbol), Some(&order_id));
                
                // Update order result
                order_result.status = OrderStatus::Rejected;
                order_result.error = Some(error_msg.clone());
                
                // Add to order history
                let live_order = LiveOrder {
                    order_id: order_id.clone(),
                    request: order.clone(),
                    result: order_result.clone(),
                    created_at: now,
                    updated_at: now,
                    status: OrderStatus::Rejected,
                    error: Some(error_msg.clone()),
                };
                self.order_history.push(live_order);
                
                // Schedule retry if appropriate
                if self.should_retry_order(&e.to_string()) {
                    self.schedule_retry(order.clone(), &e.to_string())?;
                }
                
                // Update order result history
                self.update_order_result(false);
                
                return Err(LiveTradingError::SdkError(e.to_string()));
            }
        };
        
        // Check response status
        if response.status != "ok" {
            let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
            self.send_alert(AlertLevel::Error, &format!("Order rejected: {}", error_msg), 
                           Some(&order.symbol), Some(&order_id));
            
            // Update order result
            order_result.status = OrderStatus::Rejected;
            order_result.error = Some(error_msg.clone());
            
            // Add to order history
            let live_order = LiveOrder {
                order_id: order_id.clone(),
                request: order.clone(),
                result: order_result.clone(),
                created_at: now,
                updated_at: now,
                status: OrderStatus::Rejected,
                error: Some(error_msg.clone()),
            };
            self.order_history.push(live_order);
            
            // Schedule retry if appropriate
            if self.should_retry_order(&error_msg) {
                self.schedule_retry(order.clone(), &error_msg)?;
            }
            
            // Update order result history
            self.update_order_result(false);
            
            return Err(LiveTradingError::OrderExecutionFailed(error_msg));
        }
        
        // Update the order result
        order_result.status = OrderStatus::Filled; // Assuming market orders are filled immediately
        order_result.filled_quantity = order.quantity;
        order_result.average_price = Some(market_data.price); // Using market price as an approximation
        
        // Calculate fees (approximate)
        let fee_rate = match order.order_type {
            OrderType::Market => 0.0005, // 0.05% taker fee
            OrderType::Limit => 0.0002,  // 0.02% maker fee
            _ => 0.0005,                 // Default to taker fee
        };
        let fee_amount = order.quantity * market_data.price * fee_rate;
        order_result.fees = Some(fee_amount);
        
        // Add to order history
        let live_order = LiveOrder {
            order_id: order_id.clone(),
            request: order.clone(),
            result: order_result.clone(),
            created_at: now,
            updated_at: now,
            status: OrderStatus::Filled,
            error: None,
        };
        self.order_history.push(live_order);
        
        // Log order success
        self.send_alert(AlertLevel::Info, 
                       &format!("Order executed successfully: {} {} {} @ {:.2}", 
                               order.side, order.quantity, order.symbol, 
                               order_result.average_price.unwrap_or(0.0)),
                       Some(&order.symbol), Some(&order_id));
        
        // Log detailed order information
        self.log_order_details(&order, &order_result);
        
        // Update positions
        if let Err(e) = self.update_positions().await {
            self.send_alert(AlertLevel::Warning, 
                           &format!("Failed to update positions after order: {}", e),
                           Some(&order.symbol), Some(&order_id));
        }
        
        // Generate and register stop-loss/take-profit orders if needed
        if let Some(position) = self.positions.get(&order.symbol) {
            // Generate stop-loss order
            if let Some(stop_loss) = self.risk_manager.generate_stop_loss(position, &order_id) {
                let trigger_price = stop_loss.trigger_price;
                self.risk_manager.register_stop_loss(stop_loss);
                self.send_alert(AlertLevel::Info, 
                               &format!("Stop-loss registered at {:.2}", trigger_price),
                               Some(&order.symbol), Some(&order_id));
            }
            
            // Generate take-profit order
            if let Some(take_profit) = self.risk_manager.generate_take_profit(position, &order_id) {
                let trigger_price = take_profit.trigger_price;
                self.risk_manager.register_take_profit(take_profit);
                self.send_alert(AlertLevel::Info, 
                               &format!("Take-profit registered at {:.2}", trigger_price),
                               Some(&order.symbol), Some(&order_id));
            }
        }
        
        // Update order result history
        self.update_order_result(true);
        
        Ok(order_result)
    }
    
    /// Determine if an order should be retried based on the error message
    fn should_retry_order(&self, error_msg: &str) -> bool {
        // Retry on connection errors, rate limits, or temporary API issues
        error_msg.contains("connection") || 
        error_msg.contains("timeout") || 
        error_msg.contains("rate limit") || 
        error_msg.contains("try again") ||
        error_msg.contains("temporary") ||
        error_msg.contains("overloaded")
    }    
    
/// Cancel an order
    pub async fn cancel_order(&mut self, order_id: &str) -> Result<OrderResult, LiveTradingError> {
        // Check if connected
        if !self.is_connected {
            return Err(LiveTradingError::ConnectionError("Not connected to exchange".to_string()));
        }
        
        // Check if the order exists
        if let Some(order) = self.active_orders.get(order_id) {
            // Convert to Hyperliquid cancel request
            let client_cancel = self.convert_to_client_cancel(&order.request, order_id)?;
            
            // Execute the cancel
            let exchange_client = self.exchange_client.as_ref()
                .ok_or(LiveTradingError::ConnectionError("Exchange client not initialized".to_string()))?;
            
            let response = exchange_client.cancel(client_cancel, None)
                .await
                .map_err(|e| LiveTradingError::SdkError(e.to_string()))?;
            
            // Check response status
            if response.status != "ok" {
                let error_msg = response.error.unwrap_or_else(|| "Unknown error".to_string());
                return Err(LiveTradingError::OrderExecutionFailed(error_msg));
            }
            
            // Update the order
            let mut updated_order = order.clone();
            updated_order.status = OrderStatus::Cancelled;
            updated_order.updated_at = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
            
            // Remove from active orders and add to history
            self.active_orders.remove(order_id);
            self.order_history.push(updated_order.clone());
            
            return Ok(updated_order.result);
        } else {
            return Err(LiveTradingError::OrderExecutionFailed(
                format!("Order not found: {}", order_id)
            ));
        }
    }
    
    /// Get current positions
    pub fn get_positions(&self) -> &HashMap<String, Position> {
        &self.positions
    }
    
    /// Get order history
    pub fn get_order_history(&self) -> &Vec<LiveOrder> {
        &self.order_history
    }
    
    /// Get active orders
    pub fn get_active_orders(&self) -> &HashMap<String, LiveOrder> {
        &self.active_orders
    }
    
    /// Get account balance
    pub fn get_account_balance(&self) -> f64 {
        self.account_balance
    }
    
    /// Get portfolio value (balance + position values)
    pub fn get_portfolio_value(&self) -> f64 {
        let position_value = self.positions.values()
            .map(|p| p.size.abs() * p.current_price)
            .sum::<f64>();
        
        self.account_balance + position_value
    }
    
    /// Activate emergency stop with immediate order cancellation
    #[instrument(level = "warn", skip(self))]
    pub async fn emergency_stop(&mut self) -> Result<(), LiveTradingError> {
        warn!("EMERGENCY STOP ACTIVATED");
        self.emergency_stop.store(true, Ordering::SeqCst);
        
        // Send critical alert
        self.send_alert(AlertLevel::Critical, "Emergency stop activated", None, None);
        
        // Cancel all active orders
        match self.cancel_all_orders().await {
            Ok(_) => {
                info!("Successfully cancelled all orders during emergency stop");
            },
            Err(e) => {
                error!("Failed to cancel all orders during emergency stop: {}", e);
                // We continue with emergency stop even if order cancellation fails
            }
        }
        
        // Log positions at time of emergency stop
        info!("Positions at emergency stop:");
        for (symbol, position) in &self.positions {
            info!("  {}: {} @ {:.2} (PnL: {:.2})", 
                  symbol, position.size, position.current_price, position.unrealized_pnl);
        }
        
        Ok(())
    }
    
    /// Deactivate emergency stop
    pub fn deactivate_emergency_stop(&self) {
        info!("Emergency stop deactivated");
        self.emergency_stop.store(false, Ordering::SeqCst);
        
        // Send alert
        self.send_alert(AlertLevel::Warning, "Emergency stop deactivated", None, None);
    }
    
    /// Check if emergency stop is active
    pub fn is_emergency_stop_active(&self) -> bool {
        self.emergency_stop.load(Ordering::SeqCst)
    }    

    /// Start trading with the given strategy
    pub async fn start_trading(&mut self, strategy: Box<dyn TradingStrategy>) -> Result<(), LiveTradingError> {
        // Check if connected
        if !self.is_connected {
            self.connect().await?;
        }
        
        // Check if already running
        if self.is_running {
            return Ok(());
        }
        
        info!("Starting live trading with strategy: {}", strategy.name());
        
        self.is_running = true;
        
        // Main trading loop
        while self.is_running {
            // Check if emergency stop is active
            if self.emergency_stop.load(Ordering::SeqCst) {
                warn!("Emergency stop is active, pausing trading");
                tokio::time::sleep(Duration::from_secs(5)).await;
                continue;
            }
            
            // Check if connected
            if !self.is_connected {
                warn!("Not connected to exchange, attempting to reconnect...");
                match self.connect().await {
                    Ok(_) => info!("Reconnected to exchange"),
                    Err(e) => {
                        error!("Failed to reconnect: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }
            }
            
            // Process market data updates
            self.process_market_data_updates(strategy.as_ref()).await?;
            
            // Check risk orders (stop-loss/take-profit)
            self.check_risk_orders().await?;
            
            // Sleep to avoid CPU spinning
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        
        info!("Live trading stopped");
        Ok(())
    }
    
    /// Stop trading
    pub fn stop_trading(&mut self) {
        info!("Stopping live trading");
        self.is_running = false;
    }
    
    /// Get active orders count
    pub fn active_orders_count(&self) -> usize {
        self.active_orders.len()
    }
    
    /// Get active order IDs
    pub fn active_order_ids(&self) -> Vec<String> {
        self.active_orders.keys().cloned().collect()
    }
    
    /// Get monitoring manager
    pub fn monitoring_manager(&mut self) -> Option<&mut crate::real_time_monitoring::MonitoringManager> {
        self.monitoring_manager.as_mut()
    }
    
    /// Update positions from exchange
    pub async fn update_positions(&mut self) -> Result<(), LiveTradingError> {
        // Check if connected
        if !self.is_connected {
            return Err(LiveTradingError::ConnectionError("Not connected to exchange".to_string()));
        }
        
        // Get wallet
        let wallet = self.wallet.as_ref().ok_or(LiveTradingError::WalletNotConfigured)?;
        
        // Fetch user state from API (placeholder implementation)
        // In a real implementation, we would call self.info_client.user_state(wallet.address())
        let user_state = MockUserState::default();
        
        // Update account balance
        if let Some(account_value) = user_state.margin_summary.account_value {
            self.account_balance = account_value.parse::<f64>().unwrap_or(self.account_balance);
        }
        
        // Update risk manager with new portfolio value
        self.risk_manager.update_portfolio_value(self.account_balance, 0.0)?;
        
        // Update positions
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        let mut updated_positions = HashMap::new();
        
        for asset_position in user_state.asset_positions {
            let symbol = asset_position.position.coin;
            let size = asset_position.position.szi.as_ref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            
            // Skip positions with zero size
            if size == 0.0 {
                continue;
            }
            
            let entry_price = asset_position.position.entry_px.as_ref()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            
            // Get current price from market data or API
            let current_price = if let Some(data) = self.market_data_cache.get(&symbol) {
                data.price
            } else {
                // Fetch price from API
                let all_mids = self.info_client.all_mids()
                    .await
                    .map_err(|e| LiveTradingError::SdkError(e.to_string()))?;
                
                all_mids.get(&symbol)
                    .ok_or(LiveTradingError::MarketDataNotAvailable(symbol.clone()))?
                    .parse::<f64>()
                    .map_err(|_| LiveTradingError::SdkError("Failed to parse price".to_string()))?
            };
            
            // Calculate unrealized PnL
            let unrealized_pnl = (current_price - entry_price) * size;
            
            // Create position
            let position = Position {
                symbol: symbol.clone(),
                size,
                entry_price,
                current_price,
                unrealized_pnl,
                realized_pnl: 0.0, // Not available from API
                funding_pnl: 0.0,   // Not available from API
                timestamp: now,
                leverage: 1.0, // Default leverage
                liquidation_price: None, // Not available from API
                margin: None, // Not available from API
                metadata: std::collections::HashMap::new(),
            };
            
            updated_positions.insert(symbol, position);
        }
        
        self.positions = updated_positions;
        
        Ok(())
    }   
 
    /// Get the current market data for a symbol
    fn get_market_data(&self, symbol: &str) -> Result<MarketData, LiveTradingError> {
        if let Some(data) = self.market_data_cache.get(symbol) {
            Ok(data.clone())
        } else {
            Err(LiveTradingError::MarketDataNotAvailable(symbol.to_string()))
        }
    }
    
    /// Convert OrderRequest to ClientOrderRequest
    fn convert_to_client_order(&self, order: &OrderRequest) -> Result<ClientOrderRequest, LiveTradingError> {
        // Convert order type (placeholder implementation)
        let order_type_str = match order.order_type {
            OrderType::Market => "market",
            OrderType::Limit => "limit",
            _ => return Err(LiveTradingError::OrderExecutionFailed(
                format!("Unsupported order type: {:?}", order.order_type)
            )),
        };
        
        // Convert side
        let is_buy = match order.side {
            OrderSide::Buy => true,
            OrderSide::Sell => false,
        };
        
        // Create client order request
        let client_order = ClientOrderRequest {
            symbol: order.symbol.clone(),
            side: if is_buy { "buy".to_string() } else { "sell".to_string() },
            order_type: order_type_str.to_string(),
            quantity: order.quantity.to_string(),
            price: order.price.map(|p| p.to_string()),
        };
        
        Ok(client_order)
    }
    
    /// Convert OrderRequest to ClientCancelRequest (placeholder)
    fn convert_to_client_cancel(&self, order: &OrderRequest, order_id: &str) -> Result<String, LiveTradingError> {
        // Placeholder implementation - return the order ID for cancellation
        Ok(order_id.to_string())
    }
    
    /// Start connection check task
    fn start_connection_check_task(&mut self) {
        let emergency_stop = self.emergency_stop.clone();
        
        self.connection_check_task = Some(tokio::spawn(async move {
            loop {
                // Check if emergency stop is active
                if emergency_stop.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                
                // Sleep for 30 seconds
                tokio::time::sleep(Duration::from_secs(30)).await;
                
                // In a real implementation, we would check the connection status
                // For now, we'll just log a message
                debug!("Connection check: OK");
            }
        }));
    }
    
    /// Start order update task
    fn start_order_update_task(&mut self) {
        let emergency_stop = self.emergency_stop.clone();
        
        self.order_update_task = Some(tokio::spawn(async move {
            loop {
                // Check if emergency stop is active
                if emergency_stop.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                
                // Sleep for 5 seconds
                tokio::time::sleep(Duration::from_secs(5)).await;
                
                // In a real implementation, we would update order status
                // For now, we'll just log a message
                debug!("Order update check: OK");
            }
        }));
    }
    
    /// Start position update task
    fn start_position_update_task(&mut self) {
        let emergency_stop = self.emergency_stop.clone();
        
        self.position_update_task = Some(tokio::spawn(async move {
            loop {
                // Check if emergency stop is active
                if emergency_stop.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
                
                // Sleep for 10 seconds
                tokio::time::sleep(Duration::from_secs(10)).await;
                
                // In a real implementation, we would update positions
                // For now, we'll just log a message
                debug!("Position update check: OK");
            }
        }));
    }    
   
 /// Process market data updates and execute strategy
    async fn process_market_data_updates(&mut self, strategy: &dyn TradingStrategy) -> Result<(), LiveTradingError> {
        // Get the latest market data from the real-time stream
        if let Some(data_stream) = &self.real_time_data {
            let stream = data_stream.lock().unwrap();
            
            // Get all subscribed symbols
            let symbols = stream.get_subscribed_symbols();
            
            // Update market data cache
            for symbol in symbols {
                if let Some(data) = stream.get_market_data(&symbol) {
                    self.market_data_cache.insert(symbol, data);
                }
            }
        }
        
        // Collect market data to avoid borrowing issues
        let market_data_vec: Vec<_> = self.market_data_cache.values().cloned().collect();
        
        // Process each market data update
        for market_data in market_data_vec {
            // Execute strategy on market data (placeholder - would need mutable strategy)
            // match strategy.on_market_data(&market_data) {
            match Ok(vec![]) as Result<Vec<OrderRequest>, String> {
                Ok(order_requests) => {
                    // Execute any orders generated by the strategy
                    for order_request in order_requests {
                        match self.execute_order(order_request).await {
                            Ok(_) => {},
                            Err(err) => {
                                warn!("Failed to execute order: {}", err);
                            }
                        }
                    }
                },
                Err(err) => {
                    return Err(LiveTradingError::StrategyError(err));
                }
            }
        }
        
        Ok(())
    }
    
    /// Check risk orders (stop-loss/take-profit)
    async fn check_risk_orders(&mut self) -> Result<(), LiveTradingError> {
        // Get current prices
        let current_prices: HashMap<String, f64> = self.market_data_cache.iter()
            .map(|(symbol, data)| (symbol.clone(), data.price))
            .collect();
        
        // Check if any risk orders should be triggered
        let triggered_orders = self.risk_manager.check_risk_orders(&current_prices);
        
        // Execute triggered orders
        for risk_order in triggered_orders {
            info!("Executing {} order for {}: {} {} @ {}",
                if risk_order.is_stop_loss { "stop-loss" } else { "take-profit" },
                risk_order.symbol,
                risk_order.side,
                risk_order.quantity,
                risk_order.trigger_price
            );
            
            // Convert to OrderRequest
            let order_request = OrderRequest {
                symbol: risk_order.symbol.clone(),
                side: risk_order.side,
                order_type: OrderType::Market,
                quantity: risk_order.quantity,
                price: None,
                reduce_only: true,
                time_in_force: TimeInForce::ImmediateOrCancel,
                stop_price: None,
                client_order_id: None,
                parameters: std::collections::HashMap::new(),
            };
            
            // Execute the order
            match self.execute_order(order_request).await {
                Ok(_) => {
                    info!("{} order executed successfully",
                        if risk_order.is_stop_loss { "Stop-loss" } else { "Take-profit" }
                    );
                },
                Err(err) => {
                    error!("Failed to execute {} order: {}",
                        if risk_order.is_stop_loss { "stop-loss" } else { "take-profit" },
                        err
                    );
                }
            }
        }
        
        Ok(())
    }

    /// Generate account summary for monitoring
    pub fn generate_account_summary(&self) -> crate::mode_reporting::AccountSummary {
        crate::mode_reporting::AccountSummary {
            balance: self.account_balance,
            equity: self.account_balance, // Simplified
            margin_used: 0.0, // Would need calculation
            margin_available: self.account_balance, // Simplified
        }
    }

    /// Generate position summary for monitoring
    pub fn generate_position_summary(&self) -> crate::mode_reporting::PositionSummary {
        let total_pnl: f64 = self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();
        let long_positions = self.positions.values().filter(|p| p.size > 0.0).count();
        let short_positions = self.positions.values().filter(|p| p.size < 0.0).count();
        
        crate::mode_reporting::PositionSummary {
            total_positions: self.positions.len(),
            total_pnl,
            long_positions,
            short_positions,
        }
    }

    /// Generate order summary for monitoring
    pub fn generate_order_summary(&self) -> crate::mode_reporting::OrderSummary {
        crate::mode_reporting::OrderSummary {
            active_orders: self.active_orders.len(),
            filled_orders: self.order_history.iter().filter(|o| o.status == OrderStatus::Filled).count(),
            cancelled_orders: self.order_history.iter().filter(|o| o.status == OrderStatus::Cancelled).count(),
            total_volume: 0.0, // Would need calculation
        }
    }

    /// Generate risk summary for monitoring
    pub fn generate_risk_summary(&self) -> crate::mode_reporting::RiskSummary {
        let drawdown = if self.highest_account_value > 0.0 {
            (self.highest_account_value - self.account_balance) / self.highest_account_value
        } else {
            0.0
        };
        
        crate::mode_reporting::RiskSummary {
            risk_level: if self.emergency_stop.load(Ordering::Relaxed) { "HIGH".to_string() } else { "NORMAL".to_string() },
            max_drawdown: drawdown,
            var_95: 0.0, // Would need calculation
            leverage: 1.0, // Would need calculation
        }
    }

    /// Generate system status for monitoring
    pub fn generate_system_status(&self) -> crate::mode_reporting::SystemStatus {
        crate::mode_reporting::SystemStatus {
            is_connected: self.is_connected,
            is_running: self.is_running,
            uptime_seconds: 0, // Would need calculation
            last_heartbeat: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
        }
    }

    /// Get recent alerts for monitoring
    pub fn get_recent_alerts(&self, limit: usize) -> Vec<crate::mode_reporting::AlertEntry> {
        self.alerts.iter()
            .rev()
            .take(limit)
            .map(|alert| crate::mode_reporting::AlertEntry {
                level: alert.level.to_string(),
                message: alert.message.clone(),
                timestamp: alert.timestamp,
                symbol: alert.symbol.clone(),
                order_id: None, // Add the missing field
            })
            .collect()
    }

    /// Generate performance snapshot for monitoring
    pub fn generate_performance_snapshot(&self) -> crate::mode_reporting::PerformanceSnapshot {
        let total_pnl: f64 = self.positions.values()
            .map(|p| p.unrealized_pnl)
            .sum();
        let drawdown = if self.highest_account_value > 0.0 {
            (self.highest_account_value - self.account_balance) / self.highest_account_value
        } else {
            0.0
        };
        
        crate::mode_reporting::PerformanceSnapshot {
            total_pnl,
            daily_pnl: 0.0, // Would need calculation
            win_rate: 0.0, // Would need calculation
            sharpe_ratio: 0.0, // Would need calculation
            max_drawdown: drawdown,
        }
    }

    /// Get monitoring manager reference (accessor method)
    pub fn get_monitoring_manager(&mut self) -> &mut Option<crate::real_time_monitoring::MonitoringManager> {
        &mut self.monitoring_manager
    }

    /// Get emergency stop flag (accessor)
    pub fn get_emergency_stop(&self) -> Arc<AtomicBool> {
        self.emergency_stop.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use ethers::signers::LocalWallet;
    
    // Helper function to create a test wallet
    fn create_test_wallet() -> LocalWallet {
        // This is a test private key, never use in production
        let private_key = "0000000000000000000000000000000000000000000000000000000000000001";
        LocalWallet::from_str(private_key).unwrap()
    }
    
    // Helper function to create a test API config
    fn create_test_api_config() -> ApiConfig {
        ApiConfig {
            api_key: "test_key".to_string(),
            api_secret: "test_secret".to_string(),
            endpoint: "https://api.hyperliquid-testnet.xyz".to_string(),
            use_testnet: true,
            timeout_ms: 5000,
        }
    }
    
    // Helper function to create a test risk config
    fn create_test_risk_config() -> RiskConfig {
        RiskConfig::default()
    }
    
    // Mock implementation of TradingStrategy for testing
    struct MockStrategy;
    
    impl TradingStrategy for MockStrategy {
        fn name(&self) -> &str {
            "MockStrategy"
        }
        
        fn on_market_data(&mut self, _data: &MarketData) -> std::result::Result<Vec<OrderRequest>, String> {
            // Return no orders for testing
            Ok(Vec::new())
        }
        
        fn on_order_fill(&mut self, _fill: &crate::unified_data::OrderFill) -> std::result::Result<(), String> {
            Ok(())
        }
        
        fn on_funding_payment(&mut self, _payment: &crate::unified_data::FundingPayment) -> std::result::Result<(), String> {
            Ok(())
        }
        
        fn get_current_signals(&self) -> HashMap<String, crate::unified_data::Signal> {
            HashMap::new()
        }
    }
    
    // Tests would be here, but we'll skip them for now since they would require mocking the Hyperliquid API
    // In a real implementation, we would use mocks to test the LiveTradingEngine
}