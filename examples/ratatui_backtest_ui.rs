use std::fs;
use std::io::stdout;
use std::path::Path;

use chrono::{DateTime, Duration as ChronoDuration, FixedOffset, TimeZone, Utc};
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use hyperliquid_rust_sdk::{BaseUrl, InfoClient};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame, Terminal,
};
use serde::{Deserialize, Serialize};
use serde_json;

use hyperliquid_backtest::prelude::*;

/// Application state
#[derive(Debug, Clone, PartialEq)]
enum AppState {
    MainMenu,
    BacktestForm,
    BacktestList,
    BacktestDashboard(usize),
    Loading,
}

/// Backtest configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestConfig {
    symbol: String,
    interval: String,
    period_days: f64,
    initial_capital: f64,
    strategy_path: String,
    maker_rate: f64,
    taker_rate: f64,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            symbol: "BTC".to_string(),
            interval: "1h".to_string(),
            period_days: 7.0,
            initial_capital: 10000.0,
            strategy_path: "strategies/sma_cross_10_30.json".to_string(),
            maker_rate: 0.0002,
            taker_rate: 0.0005,
        }
    }
}

/// Backtest result stored in history
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestHistoryEntry {
    id: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    timestamp: DateTime<Utc>,
    config: BacktestConfig,
    results: BacktestResults,
}

/// Simplified backtest results for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct BacktestResults {
    initial_capital: f64,
    final_equity: f64,
    total_return: f64,
    max_drawdown: f64,
    sharpe_ratio: f64,
    win_rate: f64,
    profit_factor: f64,
    trade_count: usize,
    total_commission: f64,
    total_funding_paid: f64,
    total_funding_received: f64,
    net_funding: f64,
}

/// Main application
#[derive(Clone)]
struct App {
    state: AppState,
    config: BacktestConfig,
    history: Vec<BacktestHistoryEntry>,
    selected_history_index: usize,
    form_field: usize,
    available_strategies: Vec<String>,
    selected_strategy: usize,
    available_intervals: Vec<String>,
    selected_interval: usize,
    error_message: Option<String>,
    loading_message: String,
    main_menu_selection: usize, // 0 = New Backtest, 1 = View History
    // String buffers for commission rate input to allow proper editing
    maker_rate_input: String,
    taker_rate_input: String,
}

impl App {
    fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        let available_intervals = vec![
            "1m".to_string(),
            "3m".to_string(),
            "5m".to_string(),
            "15m".to_string(),
            "30m".to_string(),
            "1h".to_string(),
            "2h".to_string(),
            "4h".to_string(),
            "8h".to_string(),
            "12h".to_string(),
            "1d".to_string(),
            "3d".to_string(),
            "1w".to_string(),
            "1M".to_string(),
        ];

        let selected_interval = available_intervals
            .iter()
            .position(|i| i == "1h")
            .unwrap_or(0);

        let mut app = Self {
            state: AppState::MainMenu,
            config: BacktestConfig::default(),
            history: Vec::new(),
            selected_history_index: 0,
            form_field: 0,
            available_strategies: Vec::new(),
            selected_strategy: 0,
            available_intervals,
            selected_interval,
            error_message: None,
            loading_message: String::new(),
            main_menu_selection: 0,
            maker_rate_input: "0.0002".to_string(),
            taker_rate_input: "0.0005".to_string(),
        };

        // Load available strategies
        app.load_available_strategies()?;
        
        // Load history
        app.load_history()?;

        Ok(app)
    }

    fn load_available_strategies(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let strategies_dir = Path::new("strategies");
        if !strategies_dir.exists() {
            return Ok(());
        }

        self.available_strategies.clear();
        for entry in fs::read_dir(strategies_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    self.available_strategies.push(name.to_string());
                }
            }
        }

        self.available_strategies.sort();
        
        // Set default if available
        if let Some(idx) = self.available_strategies
            .iter()
            .position(|s| s == "sma_cross_10_30.json") {
            self.selected_strategy = idx;
            self.config.strategy_path = format!("strategies/{}", self.available_strategies[idx]);
        }

        Ok(())
    }

    fn load_history(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let history_dir = Path::new("backtest_history");
        if !history_dir.exists() {
            fs::create_dir_all(history_dir)?;
            return Ok(());
        }

        self.history.clear();
        for entry in fs::read_dir(history_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(entry) = serde_json::from_str::<BacktestHistoryEntry>(&content) {
                        self.history.push(entry);
                    }
                }
            }
        }

        // Sort by timestamp (newest first)
        self.history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        Ok(())
    }

    fn save_backtest_result(&self, result: BacktestHistoryEntry) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let history_dir = Path::new("backtest_history");
        if !history_dir.exists() {
            fs::create_dir_all(history_dir)?;
        }

        let filename = format!(
            "{}_{}_{}_{}.json",
            result.timestamp.format("%Y-%m-%d_%H-%M-%S"),
            result.config.symbol,
            result.config.interval,
            result.id
        );

        let filepath = history_dir.join(filename);
        let json = serde_json::to_string_pretty(&result)?;
        fs::write(filepath, json)?;

        Ok(())
    }

    async fn run_backtest(&mut self) -> std::result::Result<BacktestHistoryEntry, Box<dyn std::error::Error>> {
        self.state = AppState::Loading;
        self.loading_message = "Fetching data from Hyperliquid API...".to_string();

        // Calculate time range
        let end_time = Utc::now();
        let start_time = end_time - ChronoDuration::days(self.config.period_days as i64);
        let start_timestamp = start_time.timestamp_millis() as u64;
        let end_timestamp = end_time.timestamp_millis() as u64;

        // Initialize Hyperliquid client
        self.loading_message = format!("Connecting to Hyperliquid API...");
        let info_client = InfoClient::new(None, Some(BaseUrl::Mainnet)).await
            .map_err(|e| format!("Failed to connect to Hyperliquid API: {}", e))?;

        // Fetch OHLCV data
        self.loading_message = format!(
            "Fetching {} {} data for {} days...",
            self.config.symbol, self.config.interval, self.config.period_days
        );
        let candles = info_client
            .candles_snapshot(
                self.config.symbol.clone(),
                self.config.interval.clone(),
                start_timestamp,
                end_timestamp,
            )
            .await
            .map_err(|e| format!("Failed to fetch data: {}", e))?;

        if candles.is_empty() {
            return Err("No data received from API".into());
        }

        // Convert candles to internal format
        self.loading_message = "Processing data...".to_string();
        let mut datetime = Vec::new();
        let mut open = Vec::new();
        let mut high = Vec::new();
        let mut low = Vec::new();
        let mut close = Vec::new();
        let mut volume = Vec::new();

        for candle in &candles {
            let timestamp = Utc.timestamp_millis_opt(candle.time_open as i64)
                .unwrap()
                .with_timezone(&FixedOffset::east_opt(0).unwrap());

            datetime.push(timestamp);
            open.push(candle.open.parse::<f64>().unwrap_or(0.0));
            high.push(candle.high.parse::<f64>().unwrap_or(0.0));
            low.push(candle.low.parse::<f64>().unwrap_or(0.0));
            close.push(candle.close.parse::<f64>().unwrap_or(0.0));
            volume.push(candle.vlm.parse::<f64>().unwrap_or(0.0));
        }

        // Create internal data structure
        let data = HyperliquidData::with_ohlc_data(
            self.config.symbol.clone(),
            datetime,
            open,
            high,
            low,
            close,
            volume,
        )?;

        // Load strategy config
        self.loading_message = "Loading strategy configuration...".to_string();
        let strategy_json = fs::read_to_string(&self.config.strategy_path)
            .map_err(|e| format!("Failed to read strategy file: {}", e))?;
        
        let strategy_config: serde_json::Value = serde_json::from_str(&strategy_json)
            .map_err(|e| format!("Failed to parse strategy JSON: {}", e))?;

        let fast_period = strategy_config["parameters"]["fast_period"]["Number"]
            .as_f64()
            .unwrap_or(10.0) as usize;
        let slow_period = strategy_config["parameters"]["slow_period"]["Number"]
            .as_f64()
            .unwrap_or(30.0) as usize;
        
        let strategy_name = strategy_config["name"]
            .as_str()
            .unwrap_or("Unknown Strategy")
            .to_string();

        // Create strategy
        self.loading_message = "Creating strategy...".to_string();
        let rs_data = data.to_rs_backtester_data();
        let strategy = enhanced_sma_cross(
            rs_data,
            fast_period,
            slow_period,
            Default::default(),
        );

        // Create commission - try using available methods
        // Note: HyperliquidCommission may need to be imported from a different module
        // For now, use default and adjust if needed
        let commission = HyperliquidCommission::default();

        // Run backtest
        self.loading_message = "Running backtest...".to_string();
        let mut backtest = HyperliquidBacktest::new(
            data.clone(),
            strategy_name,
            self.config.initial_capital,
            commission.clone(),
        );

        backtest.initialize_base_backtest()?;
        backtest.calculate_with_funding()?;

        // Get results
        self.loading_message = "Generating report...".to_string();
        let report = backtest.enhanced_report()?;
        let funding_summary = report.funding_summary.clone();

        let results = BacktestResults {
            initial_capital: report.initial_capital,
            final_equity: report.final_equity,
            total_return: report.total_return,
            max_drawdown: report.max_drawdown,
            sharpe_ratio: report.sharpe_ratio,
            win_rate: report.win_rate,
            profit_factor: report.profit_factor,
            trade_count: report.trade_count,
            total_commission: report.commission_stats.total_commission,
            total_funding_paid: funding_summary.total_funding_paid,
            total_funding_received: funding_summary.total_funding_received,
            net_funding: funding_summary.net_funding,
        };

        let entry = BacktestHistoryEntry {
            id: format!("backtest_{}", Utc::now().timestamp()),
            timestamp: Utc::now(),
            config: self.config.clone(),
            results,
        };

        // Save to file
        self.save_backtest_result(entry.clone())?;

        Ok(entry)
    }
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app
    let mut app = App::new()?;

    // Main loop
    let mut should_quit = false;
    while !should_quit {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match app.state {
                    AppState::MainMenu => {
                        match key.code {
                            KeyCode::Char('1') => {
                                app.state = AppState::BacktestForm;
                                app.form_field = 0;
                            }
                            KeyCode::Char('2') => {
                                app.state = AppState::BacktestList;
                                app.selected_history_index = 0;
                            }
                            KeyCode::Up => {
                                app.main_menu_selection = if app.main_menu_selection == 0 { 1 } else { 0 };
                            }
                            KeyCode::Down => {
                                app.main_menu_selection = (app.main_menu_selection + 1) % 2;
                            }
                            KeyCode::Enter => {
                                match app.main_menu_selection {
                                    0 => {
                                        app.state = AppState::BacktestForm;
                                        app.form_field = 0;
                                    }
                                    1 => {
                                        app.state = AppState::BacktestList;
                                        app.selected_history_index = 0;
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                should_quit = true;
                            }
                            _ => {}
                        }
                    }
                    AppState::BacktestForm => {
                        match key.code {
                            KeyCode::Tab => {
                                app.form_field = (app.form_field + 1) % 7;
                                // Sync input buffers when entering commission fields
                                if app.form_field == 5 {
                                    app.maker_rate_input = format!("{:.6}", app.config.maker_rate);
                                } else if app.form_field == 6 {
                                    app.taker_rate_input = format!("{:.6}", app.config.taker_rate);
                                }
                            }
                            KeyCode::BackTab => {
                                app.form_field = if app.form_field == 0 { 6 } else { app.form_field - 1 };
                                // Sync input buffers when entering commission fields
                                if app.form_field == 5 {
                                    app.maker_rate_input = format!("{:.6}", app.config.maker_rate);
                                } else if app.form_field == 6 {
                                    app.taker_rate_input = format!("{:.6}", app.config.taker_rate);
                                }
                            }
                            KeyCode::Up => {
                                // Always move to previous field - navigation first
                                if app.form_field > 0 {
                                    app.form_field -= 1;
                                }
                                // Sync input buffers when entering/leaving commission fields
                                if app.form_field == 5 {
                                    app.maker_rate_input = format!("{:.6}", app.config.maker_rate);
                                } else if app.form_field == 6 {
                                    app.taker_rate_input = format!("{:.6}", app.config.taker_rate);
                                }
                            }
                            KeyCode::Down => {
                                // Always move to next field - navigation first
                                app.form_field = (app.form_field + 1) % 7;
                                // Sync input buffers when entering/leaving commission fields
                                if app.form_field == 5 {
                                    app.maker_rate_input = format!("{:.6}", app.config.maker_rate);
                                } else if app.form_field == 6 {
                                    app.taker_rate_input = format!("{:.6}", app.config.taker_rate);
                                }
                            }
                            KeyCode::Left => {
                                // Change values in fields
                                match app.form_field {
                                    1 => {
                                        // Interval selection
                                        app.selected_interval = if app.selected_interval == 0 {
                                            app.available_intervals.len() - 1
                                        } else {
                                            app.selected_interval - 1
                                        };
                                        app.config.interval = app.available_intervals[app.selected_interval].clone();
                                    }
                                    2 => {
                                        // Period days - decrement
                                        if app.config.period_days > 0.0 {
                                            app.config.period_days = (app.config.period_days - 1.0).max(0.0);
                                        }
                                    }
                                    3 => {
                                        // Initial capital - decrement by 1000
                                        if app.config.initial_capital > 0.0 {
                                            app.config.initial_capital = (app.config.initial_capital - 1000.0).max(0.0);
                                        }
                                    }
                                    4 => {
                                        // Strategy selection
                                        if !app.available_strategies.is_empty() {
                                            app.selected_strategy = if app.selected_strategy == 0 {
                                                app.available_strategies.len() - 1
                                            } else {
                                                app.selected_strategy - 1
                                            };
                                            app.config.strategy_path = format!(
                                                "strategies/{}",
                                                app.available_strategies[app.selected_strategy]
                                            );
                                        }
                                    }
                                    5 => {
                                        // Maker rate - decrement by 0.0001 (allow negative)
                                        app.config.maker_rate -= 0.0001;
                                        // Update input buffer
                                        app.maker_rate_input = format!("{:.6}", app.config.maker_rate);
                                    }
                                    6 => {
                                        // Taker rate - decrement by 0.0001 (allow negative)
                                        app.config.taker_rate -= 0.0001;
                                        // Update input buffer
                                        app.taker_rate_input = format!("{:.6}", app.config.taker_rate);
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Right => {
                                // Change values in fields
                                match app.form_field {
                                    1 => {
                                        // Interval selection
                                        app.selected_interval = (app.selected_interval + 1) % app.available_intervals.len();
                                        app.config.interval = app.available_intervals[app.selected_interval].clone();
                                    }
                                    2 => {
                                        // Period days - increment
                                        if app.config.period_days < 365.0 {
                                            app.config.period_days = (app.config.period_days + 1.0).min(365.0);
                                        }
                                    }
                                    3 => {
                                        // Initial capital - increment by 1000
                                        app.config.initial_capital = (app.config.initial_capital + 1000.0).min(1_000_000_000.0);
                                    }
                                    4 => {
                                        // Strategy selection
                                        if !app.available_strategies.is_empty() {
                                            app.selected_strategy = (app.selected_strategy + 1) % app.available_strategies.len();
                                            app.config.strategy_path = format!(
                                                "strategies/{}",
                                                app.available_strategies[app.selected_strategy]
                                            );
                                        }
                                    }
                                    5 => {
                                        // Maker rate - increment by 0.0001 (allow any value)
                                        app.config.maker_rate += 0.0001;
                                        // Update input buffer
                                        app.maker_rate_input = format!("{:.6}", app.config.maker_rate);
                                    }
                                    6 => {
                                        // Taker rate - increment by 0.0001 (allow any value)
                                        app.config.taker_rate += 0.0001;
                                        // Update input buffer
                                        app.taker_rate_input = format!("{:.6}", app.config.taker_rate);
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Char(c) if app.form_field == 0 => {
                                // Symbol input
                                if c.is_alphanumeric() && app.config.symbol.len() < 10 {
                                    app.config.symbol.push(c);
                                }
                            }
                            KeyCode::Backspace if app.form_field == 0 => {
                                if !app.config.symbol.is_empty() {
                                    app.config.symbol.pop();
                                }
                            }
                            KeyCode::Char(c) if app.form_field == 2 => {
                                // Period days input
                                if c.is_ascii_digit() || (c == '.' && !app.config.period_days.to_string().contains('.')) {
                                    let current_str = {
                                        let s = app.config.period_days.to_string();
                                        // Remove trailing zeros and decimal point if whole number
                                        if s.contains('.') {
                                            s.trim_end_matches('0').trim_end_matches('.').to_string()
                                        } else {
                                            s
                                        }
                                    };
                                    
                                    let period_str = if app.config.period_days == 0.0 && c != '.' {
                                        c.to_string()
                                    } else {
                                        format!("{}{}", current_str, c)
                                    };
                                    if let Ok(period) = period_str.parse::<f64>() {
                                        if period >= 0.0 && period <= 365.0 {
                                            app.config.period_days = period;
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace if app.form_field == 2 => {
                                let period_str = {
                                    let s = app.config.period_days.to_string();
                                    if s.contains('.') {
                                        s.trim_end_matches('0').trim_end_matches('.').to_string()
                                    } else {
                                        s
                                    }
                                };
                                if period_str.len() > 1 {
                                    let new_str = &period_str[..period_str.len() - 1];
                                    app.config.period_days = new_str.parse().unwrap_or(0.0);
                                } else {
                                    app.config.period_days = 0.0;
                                }
                            }
                            KeyCode::Char(c) if app.form_field == 3 => {
                                // Initial capital input
                                if c.is_ascii_digit() || (c == '.' && !app.config.initial_capital.to_string().contains('.')) {
                                    let current_str = {
                                        let s = app.config.initial_capital.to_string();
                                        // Remove trailing zeros and decimal point if whole number
                                        if s.contains('.') {
                                            s.trim_end_matches('0').trim_end_matches('.').to_string()
                                        } else {
                                            s
                                        }
                                    };
                                    
                                    let capital_str = if app.config.initial_capital == 0.0 && c != '.' {
                                        c.to_string()
                                    } else {
                                        format!("{}{}", current_str, c)
                                    };
                                    if let Ok(capital) = capital_str.parse::<f64>() {
                                        if capital >= 0.0 && capital <= 1_000_000_000.0 {
                                            app.config.initial_capital = capital;
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace if app.form_field == 3 => {
                                let capital_str = {
                                    let s = app.config.initial_capital.to_string();
                                    if s.contains('.') {
                                        s.trim_end_matches('0').trim_end_matches('.').to_string()
                                    } else {
                                        s
                                    }
                                };
                                if capital_str.len() > 1 {
                                    let new_str = &capital_str[..capital_str.len() - 1];
                                    app.config.initial_capital = new_str.parse().unwrap_or(0.0);
                                } else {
                                    app.config.initial_capital = 0.0;
                                }
                            }
                            KeyCode::Char(c) if app.form_field == 5 => {
                                // Maker rate input - support both '.' and ',' as decimal separator, and negative values
                                let is_valid_char = c.is_ascii_digit() || c == '.' || c == ',' || c == '-';
                                if is_valid_char {
                                    let has_decimal = app.maker_rate_input.contains('.') || app.maker_rate_input.contains(',');
                                    let is_decimal_separator = c == '.' || c == ',';
                                    let is_minus = c == '-';
                                    
                                    // Handle minus sign - only at the beginning
                                    if is_minus {
                                        if app.maker_rate_input.is_empty() || app.maker_rate_input == "0" {
                                            app.maker_rate_input = "-".to_string();
                                        } else if app.maker_rate_input.starts_with('-') {
                                            // Remove minus if already present
                                            app.maker_rate_input.remove(0);
                                        } else {
                                            // Add minus at the beginning
                                            app.maker_rate_input = format!("-{}", app.maker_rate_input);
                                        }
                                    } else if is_decimal_separator {
                                        // Handle decimal separator
                                        if !has_decimal {
                                            if app.maker_rate_input.is_empty() || app.maker_rate_input == "-" {
                                                app.maker_rate_input.push_str("0.");
                                            } else {
                                                app.maker_rate_input.push('.');
                                            }
                                        }
                                        // If decimal already exists, ignore
                                    } else {
                                        // Handle digit
                                        if app.maker_rate_input == "0" || app.maker_rate_input == "-0" {
                                            app.maker_rate_input = if app.maker_rate_input.starts_with('-') {
                                                format!("-{}", c)
                                            } else {
                                                c.to_string()
                                            };
                                        } else {
                                            app.maker_rate_input.push(c);
                                        }
                                    }
                                    
                                    // Normalize comma to dot and parse
                                    let normalized_str = app.maker_rate_input.replace(',', ".");
                                    if !normalized_str.is_empty() && normalized_str != "-" {
                                        if let Ok(rate) = normalized_str.parse::<f64>() {
                                            // Allow any reasonable range (including negative for testing)
                                            if rate.abs() <= 10.0 {
                                                app.config.maker_rate = rate;
                                            } else {
                                                // Invalid, remove last character
                                                app.maker_rate_input.pop();
                                            }
                                        } else {
                                            // Invalid, remove last character
                                            app.maker_rate_input.pop();
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace if app.form_field == 5 => {
                                if !app.maker_rate_input.is_empty() {
                                    app.maker_rate_input.pop();
                                    // Parse updated string
                                    if app.maker_rate_input.is_empty() || app.maker_rate_input == "-" {
                                        app.config.maker_rate = 0.0;
                                        app.maker_rate_input = "0".to_string();
                                    } else {
                                        let normalized_str = app.maker_rate_input.replace(',', ".");
                                        if let Ok(rate) = normalized_str.parse::<f64>() {
                                            app.config.maker_rate = rate;
                                        }
                                    }
                                }
                            }
                            KeyCode::Char(c) if app.form_field == 6 => {
                                // Taker rate input - support both '.' and ',' as decimal separator, and negative values
                                let is_valid_char = c.is_ascii_digit() || c == '.' || c == ',' || c == '-';
                                if is_valid_char {
                                    let has_decimal = app.taker_rate_input.contains('.') || app.taker_rate_input.contains(',');
                                    let is_decimal_separator = c == '.' || c == ',';
                                    let is_minus = c == '-';
                                    
                                    // Handle minus sign - only at the beginning
                                    if is_minus {
                                        if app.taker_rate_input.is_empty() || app.taker_rate_input == "0" {
                                            app.taker_rate_input = "-".to_string();
                                        } else if app.taker_rate_input.starts_with('-') {
                                            // Remove minus if already present
                                            app.taker_rate_input.remove(0);
                                        } else {
                                            // Add minus at the beginning
                                            app.taker_rate_input = format!("-{}", app.taker_rate_input);
                                        }
                                    } else if is_decimal_separator {
                                        // Handle decimal separator
                                        if !has_decimal {
                                            if app.taker_rate_input.is_empty() || app.taker_rate_input == "-" {
                                                app.taker_rate_input.push_str("0.");
                                            } else {
                                                app.taker_rate_input.push('.');
                                            }
                                        }
                                        // If decimal already exists, ignore
                                    } else {
                                        // Handle digit
                                        if app.taker_rate_input == "0" || app.taker_rate_input == "-0" {
                                            app.taker_rate_input = if app.taker_rate_input.starts_with('-') {
                                                format!("-{}", c)
                                            } else {
                                                c.to_string()
                                            };
                                        } else {
                                            app.taker_rate_input.push(c);
                                        }
                                    }
                                    
                                    // Normalize comma to dot and parse
                                    let normalized_str = app.taker_rate_input.replace(',', ".");
                                    if !normalized_str.is_empty() && normalized_str != "-" {
                                        if let Ok(rate) = normalized_str.parse::<f64>() {
                                            // Allow any reasonable range (including negative for testing)
                                            if rate.abs() <= 10.0 {
                                                app.config.taker_rate = rate;
                                            } else {
                                                // Invalid, remove last character
                                                app.taker_rate_input.pop();
                                            }
                                        } else {
                                            // Invalid, remove last character
                                            app.taker_rate_input.pop();
                                        }
                                    }
                                }
                            }
                            KeyCode::Backspace if app.form_field == 6 => {
                                if !app.taker_rate_input.is_empty() {
                                    app.taker_rate_input.pop();
                                    // Parse updated string
                                    if app.taker_rate_input.is_empty() || app.taker_rate_input == "-" {
                                        app.config.taker_rate = 0.0;
                                        app.taker_rate_input = "0".to_string();
                                    } else {
                                        let normalized_str = app.taker_rate_input.replace(',', ".");
                                        if let Ok(rate) = normalized_str.parse::<f64>() {
                                            app.config.taker_rate = rate;
                                        }
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                // Run backtest - spawn async task
                                app.error_message = None;
                                let mut app_clone = app.clone();
                                let rt = tokio::runtime::Runtime::new().unwrap();
                                match rt.block_on(app_clone.run_backtest()) {
                                    Ok(entry) => {
                                        app.history.insert(0, entry);
                                        app.state = AppState::BacktestDashboard(0);
                                    }
                                    Err(e) => {
                                        app.error_message = Some(e.to_string());
                                        app.state = AppState::BacktestForm;
                                    }
                                }
                            }
                            KeyCode::Esc => {
                                app.state = AppState::MainMenu;
                                app.error_message = None;
                            }
                            _ => {}
                        }
                    }
                    AppState::BacktestList => {
                        match key.code {
                            KeyCode::Up => {
                                if !app.history.is_empty() {
                                    app.selected_history_index = if app.selected_history_index == 0 {
                                        app.history.len() - 1
                                    } else {
                                        app.selected_history_index - 1
                                    };
                                }
                            }
                            KeyCode::Down => {
                                if !app.history.is_empty() {
                                    app.selected_history_index = (app.selected_history_index + 1) % app.history.len();
                                }
                            }
                            KeyCode::Enter => {
                                if !app.history.is_empty() {
                                    app.state = AppState::BacktestDashboard(app.selected_history_index);
                                }
                            }
                            KeyCode::Esc | KeyCode::Char('q') => {
                                app.state = AppState::MainMenu;
                            }
                            _ => {}
                        }
                    }
                    AppState::BacktestDashboard(idx) => {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('q') => {
                                if app.history.is_empty() {
                                    app.state = AppState::MainMenu;
                                } else {
                                    app.state = AppState::BacktestList;
                                    app.selected_history_index = idx.min(app.history.len() - 1);
                                }
                            }
                            _ => {}
                        }
                    }
                    AppState::Loading => {
                        // Loading state - no input
                    }
                }
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    Ok(())
}

fn ui(f: &mut Frame, app: &App) {
    match app.state {
        AppState::MainMenu => render_main_menu(f, app),
        AppState::BacktestForm => render_backtest_form(f, app),
        AppState::BacktestList => render_backtest_list(f, app),
        AppState::BacktestDashboard(idx) => render_backtest_dashboard(f, app, idx),
        AppState::Loading => render_loading(f, app),
    }
}

fn render_main_menu(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    let title = Paragraph::new("Hyperliquid Backtest UI")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let menu_items = vec![
        if app.main_menu_selection == 0 {
            "> 1. New Backtest"
        } else {
            "  1. New Backtest"
        },
        if app.main_menu_selection == 1 {
            "> 2. View History"
        } else {
            "  2. View History"
        },
        "",
        "Press Enter to select, ↑↓ to navigate, 'q' or Esc to quit",
    ];

    let menu = Paragraph::new(menu_items.join("\n"))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title("Main Menu"));
    f.render_widget(menu, chunks[1]);

    let footer = Paragraph::new("Use ↑↓ or number keys (1/2) to navigate")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[2]);
}

fn render_backtest_form(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    let title = Paragraph::new("New Backtest Configuration")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    let form_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(chunks[1]);

    let symbol_str = app.config.symbol.clone();
    let interval_str = app.config.interval.clone();
    let period_str = app.config.period_days.to_string();
    let capital_str = app.config.initial_capital.to_string();
    let strategy_str = app.config.strategy_path.clone();
    // Use input buffers for commission rates to show exactly what user types
    let maker_str = if app.form_field == 5 {
        // Show current input when field is selected
        app.maker_rate_input.clone()
    } else {
        // Show formatted value when not selected
        if app.config.maker_rate.abs() >= 0.001 {
            format!("{:.4}", app.config.maker_rate)
        } else {
            format!("{:.6}", app.config.maker_rate).trim_end_matches('0').trim_end_matches('.').to_string()
        }
    };
    let taker_str = if app.form_field == 6 {
        // Show current input when field is selected
        app.taker_rate_input.clone()
    } else {
        // Show formatted value when not selected
        if app.config.taker_rate.abs() >= 0.001 {
            format!("{:.4}", app.config.taker_rate)
        } else {
            format!("{:.6}", app.config.taker_rate).trim_end_matches('0').trim_end_matches('.').to_string()
        }
    };
    
    let fields = vec![
        ("Symbol", symbol_str.as_str(), 0),
        ("Interval", interval_str.as_str(), 1),
        ("Period (days)", period_str.as_str(), 2),
        ("Initial Capital", capital_str.as_str(), 3),
        ("Strategy", strategy_str.as_str(), 4),
        ("Maker Rate", maker_str.as_str(), 5),
        ("Taker Rate", taker_str.as_str(), 6),
    ];

    for (idx, (label, value, field_idx)) in fields.iter().enumerate() {
        let is_selected = app.form_field == *field_idx;
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let field_text = if *field_idx == 1 && is_selected {
            // Interval selection
            format!("{}: [{}] ←→", label, value)
        } else if *field_idx == 2 && is_selected {
            // Period days - numeric input with arrows
            format!("{}: [{}] ←→", label, value)
        } else if *field_idx == 3 && is_selected {
            // Initial capital - numeric input with arrows
            format!("{}: [{}] ←→", label, value)
        } else if *field_idx == 4 && is_selected {
            // Strategy selection
            format!("{}: [{}] ←→", label, value)
        } else if *field_idx == 5 && is_selected {
            // Maker rate - numeric input with arrows
            format!("{}: [{}] ←→", label, value)
        } else if *field_idx == 6 && is_selected {
            // Taker rate - numeric input with arrows
            format!("{}: [{}] ←→", label, value)
        } else {
            format!("{}: {}", label, value)
        };

        let field = Paragraph::new(field_text)
            .style(style)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(field, form_chunks[idx]);
    }

    let help_text = if app.form_field == 0 {
        "Enter symbol (e.g., BTC, ETH) | ↑↓ to navigate fields"
    } else if app.form_field == 1 {
        "Use ←→ to change interval | ↑↓ to navigate fields"
    } else if app.form_field == 2 {
        "Type number or use ←→ to change (+1/-1) | ↑↓ to navigate fields"
    } else if app.form_field == 3 {
        "Type number or use ←→ to change (+1000/-1000) | ↑↓ to navigate fields"
    } else if app.form_field == 4 {
        "Use ←→ to change strategy | ↑↓ to navigate fields"
    } else if app.form_field == 5 {
        "Type number (e.g., 0.01, 0.001, -0.025) . or , for decimal, - for negative | ←→ to adjust"
    } else if app.form_field == 6 {
        "Type number (e.g., 0.01, 0.001, -0.025) . or , for decimal, - for negative | ←→ to adjust"
    } else {
        "Press Enter to run backtest, Esc to cancel"
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title("Help"));
    f.render_widget(help, form_chunks[7]);

    if let Some(ref error) = app.error_message {
        let error_text = Paragraph::new(format!("Error: {}", error))
            .style(Style::default().fg(Color::Red))
            .wrap(Wrap { trim: true });
        f.render_widget(Clear, f.size());
        let error_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(40), Constraint::Length(5), Constraint::Percentage(40)])
            .split(f.size());
        f.render_widget(error_text, error_chunks[1]);
    }
}

fn render_backtest_list(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    let title = Paragraph::new("Backtest History")
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    if app.history.is_empty() {
        let empty_msg = Paragraph::new("No backtests found. Create a new backtest from the main menu.")
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(empty_msg, chunks[1]);
    } else {
        let items: Vec<ListItem> = app.history
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = idx == app.selected_history_index;
                let style = if is_selected {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };

                let text = format!(
                    "{} | {} | {} | Return: {:.2}% | Sharpe: {:.2}",
                    entry.timestamp.format("%Y-%m-%d %H:%M"),
                    entry.config.symbol,
                    entry.config.interval,
                    entry.results.total_return * 100.0,
                    entry.results.sharpe_ratio
                );
                ListItem::new(text).style(style)
            })
            .collect();

        let mut list_state = ListState::default();
        list_state.select(Some(app.selected_history_index));
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Backtests"))
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        f.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    let footer = Paragraph::new("↑↓ to navigate | Enter to view | Esc to go back")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[2]);
}

fn render_backtest_dashboard(f: &mut Frame, app: &App, idx: usize) {
    if idx >= app.history.len() {
        return;
    }

    let entry = &app.history[idx];
    let results = &entry.results;

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(10),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(f.size());

    // Title
    let title = Paragraph::new(format!(
        "Backtest Results - {} | {} | {}",
        entry.config.symbol,
        entry.config.interval,
        entry.timestamp.format("%Y-%m-%d %H:%M")
    ))
    .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
    .alignment(Alignment::Center)
    .block(Block::default().borders(Borders::ALL));
    f.render_widget(title, chunks[0]);

    // Metrics
    let metrics_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let left_metrics = vec![
        format!("Initial Capital: ${:.2}", results.initial_capital),
        format!("Final Equity: ${:.2}", results.final_equity),
        format!("Total Return: {:.2}%", results.total_return * 100.0),
        format!("Max Drawdown: {:.2}%", results.max_drawdown * 100.0),
        format!("Sharpe Ratio: {:.2}", results.sharpe_ratio),
    ];

    let right_metrics = vec![
        format!("Win Rate: {:.2}%", results.win_rate * 100.0),
        format!("Profit Factor: {:.2}", results.profit_factor),
        format!("Trade Count: {}", results.trade_count),
        format!("Total Commission: ${:.2}", results.total_commission),
        format!("Net Funding: ${:.2}", results.net_funding),
    ];

    let left_block = Paragraph::new(left_metrics.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Performance Metrics"));
    f.render_widget(left_block, metrics_chunks[0]);

    let right_block = Paragraph::new(right_metrics.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Trading Metrics"));
    f.render_widget(right_block, metrics_chunks[1]);

    // Equity chart (simple ASCII representation)
    let chart_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(10), Constraint::Min(0)])
        .split(chunks[2]);

    let return_pct = results.total_return;
    let chart_height = 8;
    let chart_width = 50;
    let mut chart_lines = vec![String::new(); chart_height];

    // Simple bar chart representation
    let bar_width = ((return_pct.abs() * 100.0) as usize).min(chart_width);
    let bar_char = if return_pct >= 0.0 { '█' } else { '▓' };
    let bar_color = if return_pct >= 0.0 { Color::Green } else { Color::Red };

    for i in 0..chart_height {
        let line = if i < chart_height / 2 {
            format!("{:>10}│{}", "", "─".repeat(chart_width))
        } else if i == chart_height / 2 {
            format!("{:>10}│{} {}", "0%", bar_char.to_string().repeat(bar_width), format!("{:.2}%", return_pct * 100.0))
        } else {
            format!("{:>10}│{}", "", "─".repeat(chart_width))
        };
        chart_lines.push(line);
    }

    let chart_text = chart_lines.join("\n");
    let chart = Paragraph::new(chart_text)
        .style(Style::default().fg(bar_color))
        .block(Block::default().borders(Borders::ALL).title("Return Visualization"));
    f.render_widget(chart, chart_chunks[0]);

    // Configuration details
    let config_text = vec![
        format!("Strategy: {}", entry.config.strategy_path),
        format!("Period: {} days", entry.config.period_days),
        format!("Maker Rate: {:.4}%", entry.config.maker_rate * 100.0),
        format!("Taker Rate: {:.4}%", entry.config.taker_rate * 100.0),
        format!("Funding Paid: ${:.2}", results.total_funding_paid),
        format!("Funding Received: ${:.2}", results.total_funding_received),
    ];

    let config_block = Paragraph::new(config_text.join("\n"))
        .block(Block::default().borders(Borders::ALL).title("Configuration & Funding"));
    f.render_widget(config_block, chart_chunks[1]);

    // Footer
    let footer = Paragraph::new("Press Esc or 'q' to go back")
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[3]);
}

fn render_loading(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Length(3),
            Constraint::Percentage(40),
        ])
        .split(f.size());

    let loading_text = Paragraph::new(app.loading_message.clone())
        .alignment(Alignment::Center)
        .style(Style::default().fg(Color::Yellow))
        .block(Block::default().borders(Borders::ALL).title("Running Backtest"));
    f.render_widget(loading_text, chunks[1]);
}

