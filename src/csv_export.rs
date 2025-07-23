//! CSV export functionality for Hyperliquid backtesting

use crate::data::HyperliquidData;
use crate::errors::{HyperliquidBacktestError, Result};
use crate::backtest::{HyperliquidBacktest, FundingPayment};
use std::fs::File;
use std::io::Write;

/// Export funding payments to CSV
pub fn export_funding_payments_to_csv(payments: &[FundingPayment], file_path: &str) -> Result<()> {
    let mut file = File::create(file_path)
        .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to create file {}: {}", file_path, e)))?;
    
    // Write header
    writeln!(file, "timestamp,funding_rate,position_size,price,payment_amount")
        .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write header: {}", e)))?;
    
    // Write payment data
    for payment in payments {
        let timestamp = payment.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        
        writeln!(
            file,
            "{},{:.8},{},{:.2},{:.2}",
            timestamp,
            payment.funding_rate,
            payment.position_size,
            payment.mark_price,
            payment.payment_amount
        ).map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write payment data: {}", e)))?;
    }
    
    Ok(())
}

/// Export funding rate history to CSV
pub fn export_funding_rate_history(data: &HyperliquidData, file_path: &str) -> Result<()> {
    let mut file = File::create(file_path)
        .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to create file {}: {}", file_path, e)))?;
    
    // Write header
    writeln!(file, "timestamp,funding_rate")
        .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write header: {}", e)))?;
    
    // Write funding rate data
    for i in 0..data.datetime.len() {
        let timestamp = data.datetime[i].format("%Y-%m-%d %H:%M:%S").to_string();
        let funding_rate = data.funding_rates[i];
        
        // Only include non-NaN funding rates
        if !funding_rate.is_nan() {
            writeln!(
                file,
                "{},{:.8}",
                timestamp,
                funding_rate
            ).map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write funding rate data: {}", e)))?;
        }
    }
    
    Ok(())
}

/// Enhanced CSV export trait for HyperliquidBacktest
pub trait EnhancedCsvExport {
    /// Export backtest results to CSV
    fn export_to_csv(&self, file_path: &str) -> Result<()>;
    
    /// Export backtest results with extended data to CSV
    fn export_to_csv_extended(&self, file_path: &str, include_funding: bool, include_trading: bool, include_total: bool) -> Result<()>;
}

/// Extended CSV export trait for HyperliquidBacktest
pub trait EnhancedCsvExportExt {
    /// Export backtest results with funding data to CSV
    fn export_to_csv(&self, file_path: &str) -> Result<()>;
    
    /// Export backtest results with extended data to CSV
    fn export_to_csv_extended(&self, file_path: &str, include_funding: bool, include_trading: bool, include_total: bool) -> Result<()>;
}

impl EnhancedCsvExport for HyperliquidBacktest {
    fn export_to_csv(&self, file_path: &str) -> Result<()> {
        let mut file = File::create(file_path)
            .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to create file {}: {}", file_path, e)))?;
        
        // Write header
        writeln!(file, "timestamp,price,position,equity,funding_rate,funding_pnl")
            .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write header: {}", e)))?;
        
        // Write data rows
        for i in 0..self.data().len() {
            let timestamp = self.data().datetime[i].format("%Y-%m-%d %H:%M:%S").to_string();
            let price = self.data().close[i];
            let position = 0.0; // Placeholder, would be populated from actual position data
            let equity = self.initial_capital() + self.funding_pnl[i]; // Simplified equity calculation
            let funding_rate = if i < self.data().funding_rates.len() {
                self.data().funding_rates[i]
            } else {
                f64::NAN
            };
            
            writeln!(
                file,
                "{},{:.2},{:.2},{:.2},{:.8},{:.2}",
                timestamp,
                price,
                position,
                equity,
                if funding_rate.is_nan() { 0.0 } else { funding_rate },
                self.funding_pnl[i]
            ).map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write data row: {}", e)))?;
        }
        
        Ok(())
    }
    
    fn export_to_csv_extended(&self, file_path: &str, include_funding: bool, include_trading: bool, include_total: bool) -> Result<()> {
        let mut file = File::create(file_path)
            .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to create file {}: {}", file_path, e)))?;
        
        // Build header based on included data
        let mut header = String::from("timestamp,price,position,equity");
        
        if include_funding {
            header.push_str(",funding_rate,funding_pnl");
        }
        
        if include_trading {
            header.push_str(",trading_pnl");
        }
        
        if include_total {
            header.push_str(",total_pnl");
        }
        
        // Write header
        writeln!(file, "{}", header)
            .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write header: {}", e)))?;
        
        // Write data rows
        for i in 0..self.data().len() {
            let timestamp = self.data().datetime[i].format("%Y-%m-%d %H:%M:%S").to_string();
            let price = self.data().close[i];
            let position = 0.0; // Placeholder, would be populated from actual position data
            let equity = self.initial_capital() + self.funding_pnl[i]; // Simplified equity calculation
            
            let mut row = format!("{},{:.2},{:.2},{:.2}", timestamp, price, position, equity);
            
            if include_funding {
                let funding_rate = if i < self.data().funding_rates.len() {
                    self.data().funding_rates[i]
                } else {
                    f64::NAN
                };
                
                row.push_str(&format!(",{:.8},{:.2}", 
                    if funding_rate.is_nan() { 0.0 } else { funding_rate },
                    self.funding_pnl[i]
                ));
            }
            
            if include_trading {
                row.push_str(&format!(",{:.2}", self.trading_pnl[i]));
            }
            
            if include_total {
                let total_pnl = self.funding_pnl[i] + self.trading_pnl[i];
                row.push_str(&format!(",{:.2}", total_pnl));
            }
            
            writeln!(file, "{}", row)
                .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write data row: {}", e)))?;
        }
        
        Ok(())
    }
}

impl EnhancedCsvExportExt for HyperliquidBacktest {
    fn export_to_csv(&self, file_path: &str) -> Result<()> {
        <Self as EnhancedCsvExport>::export_to_csv(self, file_path)
    }
    
    fn export_to_csv_extended(&self, file_path: &str, include_funding: bool, include_trading: bool, include_total: bool) -> Result<()> {
        <Self as EnhancedCsvExport>::export_to_csv_extended(self, file_path, include_funding, include_trading, include_total)
    }
}

/// Strategy comparison data for multiple backtests
pub struct StrategyComparisonData {
    /// List of backtests to compare
    pub strategies: Vec<HyperliquidBacktest>,
}

impl StrategyComparisonData {
    /// Export strategy comparison data to CSV
    pub fn export_to_csv(&self, file_path: &str) -> Result<()> {
        let mut file = File::create(file_path)
            .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to create file {}: {}", file_path, e)))?;
        
        // Build header with strategy names
        let mut header = String::from("timestamp,price");
        
        for strategy in &self.strategies {
            let strategy_name = strategy.strategy_name().replace(" ", "_");
            header.push_str(&format!(",{}_position,{}_equity", strategy_name, strategy_name));
        }
        
        // Write header
        writeln!(file, "{}", header)
            .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write header: {}", e)))?;
        
        // Ensure all strategies have the same data length
        if self.strategies.is_empty() {
            return Ok(());
        }
        
        let data_len = self.strategies[0].data().len();
        
        // Write data rows
        for i in 0..data_len {
            let timestamp = self.strategies[0].data().datetime[i].format("%Y-%m-%d %H:%M:%S").to_string();
            let price = self.strategies[0].data().close[i];
            
            let mut row = format!("{},{:.2}", timestamp, price);
            
            for strategy in &self.strategies {
                let position = 0.0; // Placeholder, would be populated from actual position data
                let equity = strategy.initial_capital() + 
                    (if i < strategy.funding_pnl.len() { strategy.funding_pnl[i] } else { 0.0 }) +
                    (if i < strategy.trading_pnl.len() { strategy.trading_pnl[i] } else { 0.0 });
                
                row.push_str(&format!(",{:.2},{:.2}", position, equity));
            }
            
            writeln!(file, "{}", row)
                .map_err(|e| HyperliquidBacktestError::data_conversion(format!("Failed to write data row: {}", e)))?;
        }
        
        Ok(())
    }
}