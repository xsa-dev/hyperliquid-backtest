use std::collections::{HashMap, VecDeque};
use std::sync::atomic::Ordering;
use std::time::Duration;
use chrono::{DateTime, FixedOffset, Utc};
use log::{debug, info, warn, error};
use uuid::Uuid;
use tracing::instrument;

use crate::live_trading::{LiveTradingEngine, LiveTradingError, AlertLevel, AlertMessage};
use crate::unified_data::{OrderRequest, OrderResult};

impl LiveTradingEngine {
    /// Initialize safety mechanisms
    pub async fn init_safety_mechanisms(&mut self) -> Result<(), LiveTradingError> {
        info!("Initializing live trading safety mechanisms");
        
        // Start alert processing task
        self.start_alert_processing_task();
        
        // Start order retry task
        self.start_retry_task();
        
        // Start monitoring task
        self.start_monitoring_task();
        
        Ok(())
    }
    
    /// Send an alert message
    pub fn send_alert(&self, level: AlertLevel, message: &str, symbol: Option<&str>, order_id: Option<&str>) {
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        let alert = AlertMessage {
            level,
            message: message.to_string(),
            timestamp: now,
            symbol: symbol.map(|s| s.to_string()),
            order_id: order_id.map(|id| id.to_string()),
        };
        
        // Log the alert
        match level {
            AlertLevel::Info => info!("ALERT [INFO]: {}", message),
            AlertLevel::Warning => warn!("ALERT [WARNING]: {}", message),
            AlertLevel::Error => error!("ALERT [ERROR]: {}", message),
            AlertLevel::Critical => error!("ALERT [CRITICAL]: {}", message),
        }
        
        // Send the alert to the channel if available
        if let Some(sender) = &self.alert_sender {
            if let Err(e) = sender.try_send(alert) {
                error!("Failed to send alert: {}", e);
            }
        }
    }
    
    /// Start alert processing task
    fn start_alert_processing_task(&mut self) {
        if self.alert_task.is_some() {
            return;
        }
        
        let mut receiver = self.alert_receiver.take().unwrap();
        let emergency_stop = self.emergency_stop.clone();
        let config = self.safety_circuit_breaker_config.clone();
        let mut recent_critical_alerts = VecDeque::with_capacity(config.critical_alerts_window);
        
        self.alert_task = Some(tokio::spawn(async move {
            while let Some(alert) = receiver.recv().await {
                // Store the alert
                if alert.level == AlertLevel::Critical {
                    recent_critical_alerts.push_back(alert.clone());
                    
                    // Keep only the most recent alerts
                    while recent_critical_alerts.len() > config.critical_alerts_window {
                        recent_critical_alerts.pop_front();
                    }
                    
                    // Check if we need to trigger emergency stop
                    if recent_critical_alerts.len() >= config.max_critical_alerts as usize {
                        error!("SAFETY: Emergency stop triggered due to {} critical alerts", recent_critical_alerts.len());
                        emergency_stop.store(true, Ordering::SeqCst);
                    }
                }
                
                // In a real implementation, this would send alerts to external monitoring systems
                // For now, we just log them
            }
        }));
    }
    
    /// Start order retry task
    fn start_retry_task(&mut self) {
        if self.retry_task.is_some() {
            return;
        }
        
        let emergency_stop = self.emergency_stop.clone();
        let retry_policy = self.retry_policy.clone();
        // Using a simple structure instead of RetryState
        let mut pending_retries: HashMap<String, (u32, DateTime<FixedOffset>)> = HashMap::new();
        
        self.retry_task = Some(tokio::spawn(async move {
            loop {
                // Check if emergency stop is active
                if emergency_stop.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                
                // Process pending retries
                let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
                let mut to_retry = Vec::new();
                
                // Find orders that need to be retried
                for (order_id, retry_state) in &pending_retries {
                    // Access tuple fields directly (attempts, next_retry_time)
                    if now >= retry_state.1 {
                        to_retry.push(order_id.clone());
                    }
                }
                
                // In a real implementation, we would retry these orders
                // For now, we just log them
                for order_id in to_retry {
                    if let Some(retry_state) = pending_retries.get_mut(&order_id) {
                        debug!("Retrying order {}: attempt {}/{}", 
                               order_id, retry_state.0 + 1, 3);
                        
                        // Update retry state
                        // Update attempts count
                        retry_state.0 += 1;
                        
                        // Calculate next retry time with exponential backoff
                        // Calculate delay with exponential backoff
                        let delay_ms = 1000 * (2_u64.pow(retry_state.0));
                        
                        // Set next retry time
                        retry_state.1 = now + chrono::Duration::milliseconds(delay_ms as i64);
                        
                        // Check if we've reached the maximum number of attempts
                        // Check if max attempts reached
                        if retry_state.0 >= 3 {
                            warn!("Order {} retry limit reached after {} attempts", 
                                  order_id, retry_state.0);
                            pending_retries.remove(&order_id);
                        }
                    }
                }
                
                // Sleep for a short time
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        }));
    }
    
    /// Start monitoring task
    fn start_monitoring_task(&mut self) {
        if self.monitoring_task.is_some() {
            return;
        }
        
        let emergency_stop = self.emergency_stop.clone();
        let config = self.safety_circuit_breaker_config.clone();
        
        self.monitoring_task = Some(tokio::spawn(async move {
            loop {
                // Check if emergency stop is active
                if emergency_stop.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
                
                // In a real implementation, we would monitor various metrics
                // For now, we just sleep
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }));
    }
    
    /// Schedule an order for retry
    pub fn schedule_retry(&mut self, order_request: OrderRequest, error: &str) -> Result<(), LiveTradingError> {
        let order_id = Uuid::new_v4().to_string();
        let now = Utc::now().with_timezone(&FixedOffset::east_opt(0).unwrap());
        
        // Calculate initial retry time
        let next_retry = now + chrono::Duration::milliseconds(self.retry_policy.initial_delay_ms as i64);
        
        // Create retry state
        let retry_state = crate::live_trading::OrderRetryState {
            order_request,
            attempts: 1,
            last_attempt: now,
            last_error: error.to_string(),
            next_retry,
        };
        
        // Add to pending retries
        self.pending_retries.insert(order_id.clone(), retry_state);
        
        // Log the retry
        info!("Scheduled retry for order {}: {}", order_id, error);
        
        Ok(())
    }
    
    /// Cancel all active orders
    #[instrument(level = "info", skip(self), fields(orders = ?self.active_orders_count()))]
    pub async fn cancel_all_orders(&mut self) -> Result<(), LiveTradingError> {
        info!("Cancelling all active orders");
        
        let order_ids: Vec<String> = self.active_order_ids();
        let mut errors = Vec::new();
        
        for order_id in order_ids {
            match self.cancel_order(&order_id).await {
                Ok(_) => {
                    info!("Successfully cancelled order {}", order_id);
                },
                Err(e) => {
                    let error_msg = format!("Failed to cancel order {}: {}", order_id, e);
                    error!("{}", error_msg);
                    errors.push(error_msg);
                }
            }
        }
        
        if !errors.is_empty() {
            return Err(LiveTradingError::OrderCancellationFailed(
                format!("Failed to cancel {} orders: {}", errors.len(), errors.join(", "))
            ));
        }
        
        Ok(())
    }
    
    /// Check safety circuit breakers
    pub fn check_safety_circuit_breakers(&mut self) -> Result<(), LiveTradingError> {
        // Check consecutive failed orders
        if self.consecutive_failed_orders >= self.safety_circuit_breaker_config.max_consecutive_failed_orders {
            let msg = format!("Safety circuit breaker triggered: {} consecutive failed orders", 
                             self.consecutive_failed_orders);
            self.send_alert(AlertLevel::Critical, &msg, None, None);
            self.emergency_stop();
            return Err(LiveTradingError::SafetyCircuitBreaker(msg));
        }
        
        // Check order failure rate
        if self.order_result_history.len() >= self.safety_circuit_breaker_config.order_failure_rate_window {
            let failure_count = self.order_result_history.iter().filter(|&&success| !success).count();
            let failure_rate = failure_count as f64 / self.order_result_history.len() as f64;
            
            if failure_rate >= self.safety_circuit_breaker_config.max_order_failure_rate {
                let msg = format!("Safety circuit breaker triggered: {:.1}% order failure rate", 
                                 failure_rate * 100.0);
                self.send_alert(AlertLevel::Critical, &msg, None, None);
                self.emergency_stop();
                return Err(LiveTradingError::SafetyCircuitBreaker(msg));
            }
        }
        
        // Check account drawdown
        if self.account_balance < self.highest_account_value {
            let drawdown = (self.highest_account_value - self.account_balance) / self.highest_account_value;
            
            if drawdown >= self.safety_circuit_breaker_config.max_account_drawdown_pct {
                let msg = format!("Safety circuit breaker triggered: {:.1}% account drawdown", 
                                 drawdown * 100.0);
                self.send_alert(AlertLevel::Critical, &msg, None, None);
                self.emergency_stop();
                return Err(LiveTradingError::SafetyCircuitBreaker(msg));
            }
        }
        
        Ok(())
    }
    
    /// Update order result history
    pub fn update_order_result(&mut self, success: bool) {
        // Update consecutive failed orders count
        if success {
            self.consecutive_failed_orders = 0;
        } else {
            self.consecutive_failed_orders += 1;
        }
        
        // Update order result history
        self.order_result_history.push_back(success);
        
        // Keep only the most recent results
        while self.order_result_history.len() > self.safety_circuit_breaker_config.order_failure_rate_window {
            self.order_result_history.pop_front();
        }
    }
    
    /// Log detailed order information
    pub fn log_order_details(&self, order_request: &OrderRequest, order_result: &OrderResult) {
        if !self.detailed_logging {
            return;
        }
        
        info!("ORDER DETAILS:");
        info!("  ID: {}", order_result.order_id);
        info!("  Symbol: {}", order_request.symbol);
        info!("  Side: {:?}", order_request.side);
        info!("  Type: {:?}", order_request.order_type);
        info!("  Quantity: {}", order_request.quantity);
        info!("  Price: {:?}", order_request.price);
        info!("  Status: {:?}", order_result.status);
        info!("  Filled Quantity: {}", order_result.filled_quantity);
        info!("  Average Price: {:?}", order_result.average_price);
        info!("  Fees: {:?}", order_result.fees);
        info!("  Error: {:?}", order_result.error);
        info!("  Timestamp: {}", order_result.timestamp);
    }
}

/// Order retry state
#[derive(Debug, Clone)]
struct OrderRetryState {
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