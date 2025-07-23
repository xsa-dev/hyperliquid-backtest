//! Real-time monitoring capabilities for trading systems
//! 
//! This module provides WebSocket-based real-time updates for UI, alerting system,
//! performance metrics streaming, and trade execution monitoring and analysis.

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
use chrono::{DateTime, FixedOffset, Utc};
use log::{info, error};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;
use tokio::time::sleep;
use uuid::Uuid;

use crate::mode_reporting::{
    MonitoringDashboardData, RealTimePnLReport, AlertEntry, 
    ConnectionMetrics
};
use crate::unified_data::{OrderRequest, OrderResult, OrderStatus};
use crate::trading_mode::TradingMode;
use crate::live_trading::{LiveTradingEngine, AlertLevel};

/// Error types specific to real-time monitoring
#[derive(Debug, Error)]
pub enum MonitoringError {
    /// WebSocket server error
    #[error("WebSocket server error: {0}")]
    WebSocketServerError(String),
    
    /// Client connection error
    #[error("Client connection error: {0}")]
    ClientConnectionError(String),
    
    /// Message processing error
    #[error("Message processing error: {0}")]
    MessageProcessingError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Channel error
    #[error("Channel error: {0}")]
    ChannelError(String),
}

/// WebSocket message types for real-time monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MonitoringMessage {
    /// Dashboard update
    Dashboard(MonitoringDashboardData),
    
    /// PnL update
    PnL(RealTimePnLReport),
    
    /// Alert
    Alert(AlertEntry),
    
    /// Trade execution
    TradeExecution(TradeExecutionUpdate),
    
    /// Connection status
    ConnectionStatus(ConnectionStatusUpdate),
    
    /// Performance metrics
    PerformanceMetrics(PerformanceMetricsUpdate),
    
    /// Heartbeat
    Heartbeat { timestamp: DateTime<FixedOffset> },
}

/// Trade execution update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecutionUpdate {
    /// Order ID
    pub order_id: String,
    
    /// Symbol
    pub symbol: String,
    
    /// Order status
    pub status: OrderStatus,
    
    /// Filled quantity
    pub filled_quantity: f64,
    
    /// Average price
    pub average_price: Option<f64>,
    
    /// Execution time
    pub execution_time: DateTime<FixedOffset>,
    
    /// Execution latency in milliseconds
    pub execution_latency_ms: u64,
    
    /// Error message if any
    pub error: Option<String>,
}

/// Connection status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatusUpdate {
    /// Connection status
    pub status: ConnectionStatus,
    
    /// Timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Latency in milliseconds
    pub latency_ms: u64,
    
    /// Connection ID
    pub connection_id: String,
    
    /// Error message if any
    pub error: Option<String>,
}

/// Connection status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    /// Connected
    Connected,
    
    /// Disconnected
    Disconnected,
    
    /// Reconnecting
    Reconnecting,
    
    /// Error
    Error,
}

/// Performance metrics update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetricsUpdate {
    /// Timestamp
    pub timestamp: DateTime<FixedOffset>,
    
    /// Trading mode
    pub mode: TradingMode,
    
    /// Current balance
    pub current_balance: f64,
    
    /// Daily PnL
    pub daily_pnl: f64,
    
    /// Daily PnL percentage
    pub daily_pnl_pct: f64,
    
    /// Total PnL
    pub total_pnl: f64,
    
    /// Total return percentage
    pub total_return_pct: f64,
    
    /// Win rate
    pub win_rate: f64,
    
    /// Sharpe ratio
    pub sharpe_ratio: f64,
    
    /// Maximum drawdown percentage
    pub max_drawdown_pct: f64,
    
    /// Current positions count
    pub positions_count: usize,
}

/// Client connection information
#[derive(Debug)]
struct ClientConnection {
    /// Client ID
    id: String,
    
    /// Connection timestamp
    connected_at: DateTime<FixedOffset>,
    
    /// Last heartbeat
    last_heartbeat: Instant,
    
    /// Message sender
    sender: broadcast::Sender<String>,
}

/// Real-time monitoring server
pub struct MonitoringServer {
    /// Active client connections
    clients: Arc<Mutex<HashMap<String, ClientConnection>>>,
    
    /// Broadcast channel for messages
    broadcast_tx: broadcast::Sender<MonitoringMessage>,
    
    /// Server task handle
    server_task: Option<JoinHandle<()>>,
    
    /// Is running
    is_running: Arc<AtomicBool>,
    
    /// Server port
    port: u16,
    
    /// Message history
    message_history: Arc<Mutex<VecDeque<MonitoringMessage>>>,
    
    /// Maximum message history size
    max_history_size: usize,
}

impl MonitoringServer {
    /// Create a new monitoring server
    pub fn new(port: u16) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
            broadcast_tx,
            server_task: None,
            is_running: Arc::new(AtomicBool::new(false)),
            port,
            message_history: Arc::new(Mutex::new(VecDeque::with_capacity(100))),
            max_history_size: 100,
        }
    }
    
    /// Start the monitoring server
    pub async fn start(&mut self) -> std::result::Result<(), MonitoringError> {
        if self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("Starting monitoring server on port {}", self.port);
        
        // Set running flag
        self.is_running.store(true, Ordering::SeqCst);
        
        // Clone necessary data for the server task
        let is_running = self.is_running.clone();
        let clients = self.clients.clone();
        let broadcast_tx = self.broadcast_tx.clone();
        let message_history = self.message_history.clone();
        let port = self.port;
        
        // Start server task
        self.server_task = Some(tokio::spawn(async move {
            // In a real implementation, we would start a WebSocket server here
            // For now, we'll just simulate the server behavior
            
            info!("Monitoring server started on port {}", port);
            
            // Start heartbeat task
            let clients_clone = clients.clone();
            let is_running_clone = is_running.clone();
            
            tokio::spawn(async move {
                while is_running_clone.load(Ordering::SeqCst) {
                    // Send heartbeat to all clients
                    let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
                    let heartbeat = MonitoringMessage::Heartbeat { timestamp: now };
                    
                    if let Err(e) = broadcast_tx.send(heartbeat) {
                        error!("Failed to send heartbeat: {}", e);
                    }
                    
                    // Check for stale connections
                    {
                        let mut clients_lock = clients_clone.lock().unwrap();
                        let stale_clients: Vec<String> = clients_lock.iter()
                            .filter(|(_, client)| client.last_heartbeat.elapsed() > Duration::from_secs(30))
                            .map(|(id, _)| id.clone())
                            .collect();
                        
                        // Remove stale clients
                        for client_id in stale_clients {
                            info!("Removing stale client connection: {}", client_id);
                            clients_lock.remove(&client_id);
                        }
                    } // MutexGuard is dropped here
                    
                    // Sleep for a while
                    sleep(Duration::from_secs(5)).await;
                }
            });
            
            // Main server loop
            while is_running.load(Ordering::SeqCst) {
                sleep(Duration::from_secs(1)).await;
            }
            
            info!("Monitoring server stopped");
        }));
        
        Ok(())
    }
    
    /// Stop the monitoring server
    pub async fn stop(&mut self) -> std::result::Result<(), MonitoringError> {
        if !self.is_running.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("Stopping monitoring server");
        
        // Set running flag
        self.is_running.store(false, Ordering::SeqCst);
        
        // Wait for server task to complete
        if let Some(task) = self.server_task.take() {
            task.abort();
        }
        
        // Clear clients
        let mut clients_lock = self.clients.lock().unwrap();
        clients_lock.clear();
        
        info!("Monitoring server stopped");
        
        Ok(())
    }
    
    /// Broadcast a message to all clients
    pub fn broadcast_message(&self, message: MonitoringMessage) -> std::result::Result<(), MonitoringError> {
        // Add message to history
        {
            let mut history_lock = self.message_history.lock().unwrap();
            history_lock.push_back(message.clone());
            
            // Keep history size limited
            while history_lock.len() > self.max_history_size {
                history_lock.pop_front();
            }
        }
        
        // Broadcast message
        if let Err(e) = self.broadcast_tx.send(message) {
            return Err(MonitoringError::ChannelError(format!("Failed to broadcast message: {}", e)));
        }
        
        Ok(())
    }
    
    /// Get client count
    pub fn client_count(&self) -> usize {
        let clients_lock = self.clients.lock().unwrap();
        clients_lock.len()
    }
    
    /// Get message history
    pub fn get_message_history(&self) -> Vec<MonitoringMessage> {
        let history_lock = self.message_history.lock().unwrap();
        history_lock.iter().cloned().collect()
    }
}

/// Real-time monitoring client
pub struct MonitoringClient {
    /// Client ID
    id: String,
    
    /// Server address
    server_address: String,
    
    /// Message receiver
    message_rx: Option<broadcast::Receiver<MonitoringMessage>>,
    
    /// Client task handle
    client_task: Option<JoinHandle<()>>,
    
    /// Is connected
    is_connected: Arc<AtomicBool>,
    
    /// Connection status
    connection_status: Arc<Mutex<ConnectionStatus>>,
    
    /// Last received message timestamp
    last_message: Arc<Mutex<Option<DateTime<FixedOffset>>>>,
    
    /// Message handlers
    message_handlers: Arc<Mutex<Vec<Box<dyn Fn(MonitoringMessage) + Send + Sync>>>>,
}

impl MonitoringClient {
    /// Create a new monitoring client
    pub fn new(server_address: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            server_address: server_address.to_string(),
            message_rx: None,
            client_task: None,
            is_connected: Arc::new(AtomicBool::new(false)),
            connection_status: Arc::new(Mutex::new(ConnectionStatus::Disconnected)),
            last_message: Arc::new(Mutex::new(None)),
            message_handlers: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    /// Connect to the monitoring server
    pub async fn connect(&mut self) -> std::result::Result<(), MonitoringError> {
        if self.is_connected.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("Connecting to monitoring server at {}", self.server_address);
        
        // In a real implementation, we would connect to the WebSocket server here
        // For now, we'll just simulate the connection
        
        // Create message channel
        let (tx, rx) = broadcast::channel(100);
        self.message_rx = Some(rx);
        
        // Set connected flag
        self.is_connected.store(true, Ordering::SeqCst);
        {
            let mut status_lock = self.connection_status.lock().unwrap();
            *status_lock = ConnectionStatus::Connected;
        }
        
        // Clone necessary data for the client task
        let is_connected = self.is_connected.clone();
        let connection_status = self.connection_status.clone();
        let last_message = self.last_message.clone();
        let message_handlers = self.message_handlers.clone();
        let mut rx = match self.message_rx.take() {
            Some(rx) => rx,
            None => return Err(MonitoringError::ChannelError("Message receiver not available".to_string())),
        };
        
        // Start client task
        self.client_task = Some(tokio::spawn(async move {
            while is_connected.load(Ordering::SeqCst) {
                match rx.recv().await {
                    Ok(message) => {
                        // Update last message timestamp
                        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
                        {
                            let mut last_message_lock = last_message.lock().unwrap();
                            *last_message_lock = Some(now);
                        }
                        
                        // Process message
                        let handlers_lock = message_handlers.lock().unwrap();
                        for handler in handlers_lock.iter() {
                            handler(message.clone());
                        }
                    },
                    Err(e) => {
                        error!("Error receiving message: {}", e);
                        
                        // Update connection status
                        {
                            let mut status_lock = connection_status.lock().unwrap();
                            *status_lock = ConnectionStatus::Error;
                        }
                        
                        // Try to reconnect
                        sleep(Duration::from_secs(5)).await;
                        
                        {
                            let mut status_lock = connection_status.lock().unwrap();
                            *status_lock = ConnectionStatus::Reconnecting;
                        }
                        
                        // In a real implementation, we would reconnect to the WebSocket server here
                        sleep(Duration::from_secs(1)).await;
                        
                        {
                            let mut status_lock = connection_status.lock().unwrap();
                            *status_lock = ConnectionStatus::Connected;
                        }
                    }
                }
            }
            
            info!("Monitoring client disconnected");
        }));
        
        info!("Connected to monitoring server");
        
        Ok(())
    }
    
    /// Disconnect from the monitoring server
    pub async fn disconnect(&mut self) -> std::result::Result<(), MonitoringError> {
        if !self.is_connected.load(Ordering::SeqCst) {
            return Ok(());
        }
        
        info!("Disconnecting from monitoring server");
        
        // Set connected flag
        self.is_connected.store(false, Ordering::SeqCst);
        
        // Update connection status
        {
            let mut status_lock = self.connection_status.lock().unwrap();
            *status_lock = ConnectionStatus::Disconnected;
        }
        
        // Cancel client task
        if let Some(task) = self.client_task.take() {
            task.abort();
        }
        
        // Clear message receiver
        self.message_rx = None;
        
        info!("Disconnected from monitoring server");
        
        Ok(())
    }
    
    /// Add message handler
    pub fn add_message_handler<F>(&self, handler: F)
    where
        F: Fn(MonitoringMessage) + Send + Sync + 'static,
    {
        let mut handlers_lock = self.message_handlers.lock().unwrap();
        handlers_lock.push(Box::new(handler));
    }
    
    /// Get connection status
    pub fn connection_status(&self) -> ConnectionStatus {
        let status_lock = self.connection_status.lock().unwrap();
        status_lock.clone()
    }
    
    /// Get last message timestamp
    pub fn last_message_timestamp(&self) -> Option<DateTime<FixedOffset>> {
        let last_message_lock = self.last_message.lock().unwrap();
        last_message_lock.clone()
    }
    
    /// Is connected
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }
}

/// Real-time monitoring manager
pub struct MonitoringManager {
    /// Trading mode
    mode: TradingMode,
    
    /// Monitoring server
    server: Option<MonitoringServer>,
    
    /// Monitoring client
    client: Option<MonitoringClient>,
    
    /// Alert history
    alert_history: Vec<AlertEntry>,
    
    /// Trade execution history
    trade_execution_history: Vec<TradeExecutionUpdate>,
    
    /// Performance metrics history
    performance_metrics_history: Vec<PerformanceMetricsUpdate>,
    
    /// Connection metrics
    connection_metrics: ConnectionMetrics,
    
    /// Last dashboard update
    last_dashboard_update: Option<DateTime<FixedOffset>>,
    
    /// Dashboard update interval in seconds
    dashboard_update_interval: u64,
    
    /// Performance metrics update interval in seconds
    performance_update_interval: u64,
    
    /// Alert handlers
    alert_handlers: Vec<Box<dyn Fn(&AlertEntry) + Send + Sync>>,
    
    /// Trade execution handlers
    trade_execution_handlers: Vec<Box<dyn Fn(&TradeExecutionUpdate) + Send + Sync>>,
}

impl MonitoringManager {
    /// Create a new monitoring manager
    pub fn new(mode: TradingMode) -> Self {
        Self {
            mode,
            server: None,
            client: None,
            alert_history: Vec::new(),
            trade_execution_history: Vec::new(),
            performance_metrics_history: Vec::new(),
            connection_metrics: ConnectionMetrics {
                uptime_pct: 100.0,
                disconnection_count: 0,
                avg_reconnection_time_ms: 0.0,
                api_latency_ms: 0.0,
                ws_latency_ms: 0.0,
                order_latency_ms: 0.0,
            },
            last_dashboard_update: None,
            dashboard_update_interval: 5, // 5 seconds
            performance_update_interval: 60, // 60 seconds
            alert_handlers: Vec::new(),
            trade_execution_handlers: Vec::new(),
        }
    }
    
    /// Start monitoring server
    pub async fn start_server(&mut self, port: u16) -> std::result::Result<(), MonitoringError> {
        if self.server.is_some() {
            return Ok(());
        }
        
        let mut server = MonitoringServer::new(port);
        server.start().await?;
        
        self.server = Some(server);
        
        Ok(())
    }
    
    /// Stop monitoring server
    pub async fn stop_server(&mut self) -> std::result::Result<(), MonitoringError> {
        if let Some(server) = self.server.as_mut() {
            server.stop().await?;
        }
        
        self.server = None;
        
        Ok(())
    }
    
    /// Connect to monitoring server
    pub async fn connect_to_server(&mut self, server_address: &str) -> std::result::Result<(), MonitoringError> {
        if self.client.is_some() {
            return Ok(());
        }
        
        let mut client = MonitoringClient::new(server_address);
        client.connect().await?;
        
        // Add message handlers
        let alert_history = Arc::new(Mutex::new(self.alert_history.clone()));
        let trade_execution_history = Arc::new(Mutex::new(self.trade_execution_history.clone()));
        let performance_metrics_history = Arc::new(Mutex::new(self.performance_metrics_history.clone()));
        
        client.add_message_handler(move |message| {
            match message {
                MonitoringMessage::Alert(alert) => {
                    let mut history_lock = alert_history.lock().unwrap();
                    history_lock.push(alert);
                },
                MonitoringMessage::TradeExecution(execution) => {
                    let mut history_lock = trade_execution_history.lock().unwrap();
                    history_lock.push(execution);
                },
                MonitoringMessage::PerformanceMetrics(metrics) => {
                    let mut history_lock = performance_metrics_history.lock().unwrap();
                    history_lock.push(metrics);
                },
                _ => {}
            }
        });
        
        self.client = Some(client);
        
        Ok(())
    }
    
    /// Disconnect from monitoring server
    pub async fn disconnect_from_server(&mut self) -> std::result::Result<(), MonitoringError> {
        if let Some(client) = self.client.as_mut() {
            client.disconnect().await?;
        }
        
        self.client = None;
        
        Ok(())
    }
    
    /// Send alert
    pub fn send_alert(&mut self, level: AlertLevel, message: &str, symbol: Option<&str>, order_id: Option<&str>) -> std::result::Result<(), MonitoringError> {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        let alert = AlertEntry {
            level: level.to_string(),
            message: message.to_string(),
            timestamp: now,
            symbol: symbol.map(|s| s.to_string()),
            order_id: order_id.map(|id| id.to_string()),
        };
        
        // Add to history
        self.alert_history.push(alert.clone());
        
        // Call alert handlers
        for handler in &self.alert_handlers {
            handler(&alert);
        }
        
        // Broadcast alert if server is running
        if let Some(server) = &self.server {
            server.broadcast_message(MonitoringMessage::Alert(alert))?;
        }
        
        Ok(())
    }
    
    /// Record trade execution
    pub fn record_trade_execution(&mut self, order_request: &OrderRequest, order_result: &OrderResult, execution_latency_ms: u64) -> std::result::Result<(), MonitoringError> {
        let execution = TradeExecutionUpdate {
            order_id: order_result.order_id.clone(),
            symbol: order_request.symbol.clone(),
            status: order_result.status.clone(),
            filled_quantity: order_result.filled_quantity,
            average_price: order_result.average_price,
            execution_time: order_result.timestamp,
            execution_latency_ms,
            error: order_result.error.clone(),
        };
        
        // Add to history
        self.trade_execution_history.push(execution.clone());
        
        // Update connection metrics
        self.connection_metrics.order_latency_ms = 
            (self.connection_metrics.order_latency_ms * 0.9) + (execution_latency_ms as f64 * 0.1);
        
        // Call trade execution handlers
        for handler in &self.trade_execution_handlers {
            handler(&execution);
        }
        
        // Broadcast trade execution if server is running
        if let Some(server) = &self.server {
            server.broadcast_message(MonitoringMessage::TradeExecution(execution))?;
        }
        
        Ok(())
    }
    
    /// Update performance metrics
    pub fn update_performance_metrics(&mut self, 
                                     current_balance: f64,
                                     daily_pnl: f64,
                                     total_pnl: f64,
                                     win_rate: f64,
                                     sharpe_ratio: f64,
                                     max_drawdown_pct: f64,
                                     positions_count: usize) -> std::result::Result<(), MonitoringError> {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        // Calculate daily PnL percentage
        let daily_pnl_pct = if current_balance > 0.0 {
            daily_pnl / current_balance * 100.0
        } else {
            0.0
        };
        
        // Calculate total return percentage
        let total_return_pct = if current_balance > 0.0 {
            total_pnl / current_balance * 100.0
        } else {
            0.0
        };
        
        let metrics = PerformanceMetricsUpdate {
            timestamp: now,
            mode: self.mode,
            current_balance,
            daily_pnl,
            daily_pnl_pct,
            total_pnl,
            total_return_pct,
            win_rate,
            sharpe_ratio,
            max_drawdown_pct,
            positions_count,
        };
        
        // Add to history
        self.performance_metrics_history.push(metrics.clone());
        
        // Broadcast performance metrics if server is running
        if let Some(server) = &self.server {
            server.broadcast_message(MonitoringMessage::PerformanceMetrics(metrics))?;
        }
        
        Ok(())
    }
    
    /// Update connection metrics
    pub fn update_connection_metrics(&mut self, 
                                    uptime_pct: f64,
                                    disconnection_count: usize,
                                    avg_reconnection_time_ms: f64,
                                    api_latency_ms: f64,
                                    ws_latency_ms: f64) -> std::result::Result<(), MonitoringError> {
        self.connection_metrics = ConnectionMetrics {
            uptime_pct,
            disconnection_count,
            avg_reconnection_time_ms,
            api_latency_ms,
            ws_latency_ms,
            order_latency_ms: self.connection_metrics.order_latency_ms,
        };
        
        Ok(())
    }
    
    /// Update dashboard
    pub fn update_dashboard(&mut self, dashboard_data: MonitoringDashboardData) -> std::result::Result<(), MonitoringError> {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        self.last_dashboard_update = Some(now);
        
        // Broadcast dashboard if server is running
        if let Some(server) = &self.server {
            server.broadcast_message(MonitoringMessage::Dashboard(dashboard_data))?;
        }
        
        Ok(())
    }
    
    /// Add alert handler
    pub fn add_alert_handler<F>(&mut self, handler: F)
    where
        F: Fn(&AlertEntry) + Send + Sync + 'static,
    {
        self.alert_handlers.push(Box::new(handler));
    }
    
    /// Add trade execution handler
    pub fn add_trade_execution_handler<F>(&mut self, handler: F)
    where
        F: Fn(&TradeExecutionUpdate) + Send + Sync + 'static,
    {
        self.trade_execution_handlers.push(Box::new(handler));
    }
    
    /// Get alert history
    pub fn get_alert_history(&self) -> &[AlertEntry] {
        &self.alert_history
    }
    
    /// Get trade execution history
    pub fn get_trade_execution_history(&self) -> &[TradeExecutionUpdate] {
        &self.trade_execution_history
    }
    
    /// Get performance metrics history
    pub fn get_performance_metrics_history(&self) -> &[PerformanceMetricsUpdate] {
        &self.performance_metrics_history
    }
    
    /// Get connection metrics
    pub fn get_connection_metrics(&self) -> &ConnectionMetrics {
        &self.connection_metrics
    }
    
    /// Should update dashboard
    pub fn should_update_dashboard(&self) -> bool {
        if let Some(last_update) = self.last_dashboard_update {
            let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
            let elapsed = now.signed_duration_since(last_update).num_seconds() as u64;
            elapsed >= self.dashboard_update_interval
        } else {
            true
        }
    }
    
    /// Set dashboard update interval
    pub fn set_dashboard_update_interval(&mut self, interval_seconds: u64) {
        self.dashboard_update_interval = interval_seconds;
    }
    
    /// Set performance update interval
    pub fn set_performance_update_interval(&mut self, interval_seconds: u64) {
        self.performance_update_interval = interval_seconds;
    }
}

/// Integration with LiveTradingEngine
impl LiveTradingEngine {
    /// Initialize real-time monitoring
    pub fn init_real_time_monitoring(&mut self, port: Option<u16>) -> std::result::Result<(), MonitoringError> {
        let mut monitoring_manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Start server if port is provided
        if let Some(port) = port {
            tokio::spawn(async move {
                if let Err(e) = monitoring_manager.start_server(port).await {
                    error!("Failed to start monitoring server: {}", e);
                }
            });
        }
        
        // Store monitoring manager (this would need a setter method in LiveTradingEngine)
        // self.monitoring_manager = Some(monitoring_manager);
        
        Ok(())
    }
    
    /// Send monitoring alert
    pub fn send_monitoring_alert(&mut self, level: AlertLevel, message: &str, symbol: Option<&str>, order_id: Option<&str>) -> std::result::Result<(), MonitoringError> {
        if let Some(monitoring_manager) = self.monitoring_manager() {
            monitoring_manager.send_alert(level, message, symbol, order_id)?;
        }
        
        Ok(())
    }
    
    /// Record trade execution for monitoring
    pub fn record_trade_execution(&mut self, order_request: &OrderRequest, order_result: &OrderResult, execution_latency_ms: u64) -> std::result::Result<(), MonitoringError> {
        if let Some(monitoring_manager) = self.monitoring_manager() {
            monitoring_manager.record_trade_execution(order_request, order_result, execution_latency_ms)?;
        }
        
        Ok(())
    }
    
    /// Update performance metrics for monitoring
    pub fn update_performance_metrics(&mut self) -> std::result::Result<(), MonitoringError> {
        // Calculate metrics first
        let current_balance = 0.0; // self.account_balance;
        let daily_pnl = 0.0; // self.calculate_daily_pnl();
        let total_pnl = 0.0; // self.account_balance - self.initial_balance;
        let win_rate = 0.0; // self.calculate_win_rate();
        let sharpe_ratio = 0.0; // self.calculate_sharpe_ratio();
        let max_drawdown_pct = 0.0; // self.calculate_max_drawdown_pct();
        let positions_count = self.positions.len();
        
        if let Some(monitoring_manager) = self.get_monitoring_manager() {
            
            monitoring_manager.update_performance_metrics(
                current_balance,
                daily_pnl,
                total_pnl,
                win_rate,
                sharpe_ratio,
                max_drawdown_pct,
                positions_count
            )?;
        }
        
        Ok(())
    }
    
    /// Update connection metrics for monitoring
    pub fn update_connection_metrics(&mut self, 
                                    uptime_pct: f64,
                                    disconnection_count: usize,
                                    avg_reconnection_time_ms: f64,
                                    api_latency_ms: f64,
                                    ws_latency_ms: f64) -> std::result::Result<(), MonitoringError> {
        if let Some(monitoring_manager) = self.monitoring_manager() {
            monitoring_manager.update_connection_metrics(
                uptime_pct,
                disconnection_count,
                avg_reconnection_time_ms,
                api_latency_ms,
                ws_latency_ms
            )?;
        }
        
        Ok(())
    }
    
    /// Update monitoring dashboard
    pub fn update_monitoring_dashboard(&mut self) -> std::result::Result<(), MonitoringError> {
        // Check if we have a monitoring manager and should update
        let should_update = if let Some(monitoring_manager) = &self.monitoring_manager {
            monitoring_manager.should_update_dashboard()
        } else {
            false
        };
        
        if !should_update {
            return Ok(());
        }
        
        // Generate dashboard data
        let dashboard_data = self.generate_monitoring_dashboard_data()?;
        
        // Update dashboard
        if let Some(monitoring_manager) = &mut self.monitoring_manager {
            monitoring_manager.update_dashboard(dashboard_data)?;
        }
        
        Ok(())
    }
    
    /// Generate monitoring dashboard data
    fn generate_monitoring_dashboard_data(&self) -> std::result::Result<MonitoringDashboardData, MonitoringError> {
        // This is a simplified implementation
        // In a real implementation, we would gather all the necessary data
        
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        // Create dashboard data
        let dashboard_data = MonitoringDashboardData {
            timestamp: now,
            account_summary: self.generate_account_summary(),
            position_summary: self.generate_position_summary(),
            order_summary: self.generate_order_summary(),
            risk_summary: self.generate_risk_summary(),
            system_status: self.generate_system_status(),
            recent_alerts: self.get_recent_alerts(10),
            performance: self.generate_performance_snapshot(),
        };
        
        Ok(dashboard_data)
    }
    
    // Helper methods for generating dashboard components
    // These would be implemented in the actual LiveTradingEngine
    
    /// Calculate daily PnL
    fn calculate_daily_pnl(&self) -> f64 {
        // Simplified implementation
        0.0
    }
    
    /// Calculate win rate
    fn calculate_win_rate(&self) -> f64 {
        // Simplified implementation
        0.0
    }
    
    /// Calculate Sharpe ratio
    fn calculate_sharpe_ratio(&self) -> f64 {
        // Simplified implementation
        0.0
    }
    
    /// Calculate maximum drawdown percentage
    fn calculate_max_drawdown_pct(&self) -> f64 {
        // Simplified implementation
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_monitoring_server_creation() {
        let mut server = MonitoringServer::new(8080);
        assert_eq!(server.port, 8080);
        assert_eq!(server.client_count(), 0);
    }
    
    #[tokio::test]
    async fn test_monitoring_client_creation() {
        let client = MonitoringClient::new("ws://localhost:8080");
        assert_eq!(client.server_address, "ws://localhost:8080");
        assert_eq!(client.is_connected(), false);
        assert_eq!(client.connection_status(), ConnectionStatus::Disconnected);
    }
    
    #[tokio::test]
    async fn test_monitoring_manager_creation() {
        let manager = MonitoringManager::new(TradingMode::LiveTrade);
        assert_eq!(manager.mode, TradingMode::LiveTrade);
        assert_eq!(manager.alert_history.len(), 0);
        assert_eq!(manager.trade_execution_history.len(), 0);
    }
    
    #[tokio::test]
    async fn test_send_alert() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Send an alert
        let result = manager.send_alert(
            AlertLevel::Warning,
            "Test alert",
            Some("BTC"),
            None
        );
        
        assert!(result.is_ok());
        assert_eq!(manager.alert_history.len(), 1);
        
        let alert = &manager.alert_history[0];
        assert_eq!(alert.level, "Warning");
        assert_eq!(alert.message, "Test alert");
        assert_eq!(alert.symbol, Some("BTC".to_string()));
        assert_eq!(alert.order_id, None);
    }
    
    #[tokio::test]
    async fn test_record_trade_execution() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Create order request and result
        let order_request = OrderRequest {
            symbol: "BTC".to_string(),
            side: crate::unified_data::OrderSide::Buy,
            order_type: crate::unified_data::OrderType::Market,
            quantity: 1.0,
            price: None,
            reduce_only: false,
            time_in_force: crate::unified_data::TimeInForce::GoodTilCancelled,
        };
        
        let order_result = OrderResult {
            order_id: "test_order".to_string(),
            status: OrderStatus::Filled,
            filled_quantity: 1.0,
            average_price: Some(50000.0),
            fees: Some(25.0),
            timestamp: Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap()),
            error: None,
        };
        
        // Record trade execution
        let result = manager.record_trade_execution(&order_request, &order_result, 100);
        
        assert!(result.is_ok());
        assert_eq!(manager.trade_execution_history.len(), 1);
        
        let execution = &manager.trade_execution_history[0];
        assert_eq!(execution.order_id, "test_order");
        assert_eq!(execution.symbol, "BTC");
        assert_eq!(execution.status, OrderStatus::Filled);
        assert_eq!(execution.filled_quantity, 1.0);
        assert_eq!(execution.average_price, Some(50000.0));
        assert_eq!(execution.execution_latency_ms, 100);
        assert_eq!(execution.error, None);
    }
    
    #[tokio::test]
    async fn test_update_performance_metrics() {
        let mut manager = MonitoringManager::new(TradingMode::LiveTrade);
        
        // Update performance metrics
        let result = manager.update_performance_metrics(
            10000.0, // current_balance
            100.0,   // daily_pnl
            500.0,   // total_pnl
            0.6,     // win_rate
            1.5,     // sharpe_ratio
            5.0,     // max_drawdown_pct
            2        // positions_count
        );
        
        assert!(result.is_ok());
        assert_eq!(manager.performance_metrics_history.len(), 1);
        
        let metrics = &manager.performance_metrics_history[0];
        assert_eq!(metrics.current_balance, 10000.0);
        assert_eq!(metrics.daily_pnl, 100.0);
        assert_eq!(metrics.daily_pnl_pct, 1.0); // 100 / 10000 * 100
        assert_eq!(metrics.total_pnl, 500.0);
        assert_eq!(metrics.total_return_pct, 5.0); // 500 / 10000 * 100
        assert_eq!(metrics.win_rate, 0.6);
        assert_eq!(metrics.sharpe_ratio, 1.5);
        assert_eq!(metrics.max_drawdown_pct, 5.0);
        assert_eq!(metrics.positions_count, 2);
    }
}