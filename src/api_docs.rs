//! # API Documentation and Stability Guarantees
//!
//! This module provides comprehensive documentation for the public API and outlines
//! stability guarantees for different components of the hyperliquid-backtester crate.
//!
//! ## Public API Overview
//!
//! The hyperliquid-backtester crate provides a stable public API through the [`prelude`](crate::prelude) module.
//! All items exported in the prelude are considered part of the stable public API and follow
//! semantic versioning guarantees.
//!
//! ### Core Data Types
//!
//! #### [`HyperliquidData`](crate::data::HyperliquidData)
//! 
//! The primary data structure for market data with OHLC prices and funding rates.
//!
//! **Stability**: ✅ Stable - Fields and methods will not be removed in minor versions
//!
//! **Key Methods**:
//! - `fetch()` - Async data fetching from Hyperliquid API
//! - `to_rs_backtester_data()` - Convert to rs-backtester format
//! - `get_funding_rate_at()` - Get funding rate at specific timestamp
//! - `len()`, `is_empty()` - Data size information
//!
//! #### [`HyperliquidBacktest`](crate::backtest::HyperliquidBacktest)
//!
//! Enhanced backtesting engine with funding rate calculations.
//!
//! **Stability**: ✅ Stable - Core interface will not change in minor versions
//!
//! **Key Methods**:
//! - `new()` - Create new backtest instance
//! - `calculate_with_funding()` - Run backtest with funding calculations
//! - `enhanced_report()` - Generate comprehensive report
//! - `funding_report()` - Get funding-specific metrics
//!
//! #### [`HyperliquidCommission`](crate::backtest::HyperliquidCommission)
//!
//! Commission structure with maker/taker rates and funding settings.
//!
//! **Stability**: ✅ Stable - Structure and default values are stable
//!
//! **Key Methods**:
//! - `default()` - Standard Hyperliquid commission rates
//! - `new()` - Custom commission configuration
//! - `calculate_fee()` - Calculate trading fees
//!
//! ### Strategy Types
//!
//! #### [`HyperliquidStrategy`](crate::strategies::HyperliquidStrategy)
//!
//! Trait for implementing funding-aware strategies.
//!
//! **Stability**: ✅ Stable - Trait methods will not change in minor versions
//!
//! **Key Methods**:
//! - `funding_config()` - Get funding configuration
//! - `process_funding()` - Process funding rate signals
//! - `combine_signals()` - Combine base and funding signals
//!
//! #### Strategy Functions
//!
//! - [`funding_arbitrage_strategy()`](crate::strategies::funding_arbitrage_strategy) - ✅ Stable
//! - [`enhanced_sma_cross()`](crate::strategies::enhanced_sma_cross) - ✅ Stable
//!
//! ### Error Handling
//!
//! #### [`HyperliquidBacktestError`](crate::errors::HyperliquidBacktestError)
//!
//! Comprehensive error type with user-friendly messages.
//!
//! **Stability**: ⚠️ Semi-stable - New variants may be added, existing ones will not be removed
//!
//! **Key Methods**:
//! - `user_message()` - Get user-friendly error message with suggestions
//! - `is_recoverable()` - Check if error is recoverable
//! - `category()` - Get error category for logging
//!
//! ### Reporting and Export
//!
//! #### [`FundingReport`](crate::funding_report::FundingReport)
//!
//! Detailed funding rate analysis and metrics.
//!
//! **Stability**: ✅ Stable - Core metrics will not change
//!
//! #### [`EnhancedCsvExport`](crate::csv_export::EnhancedCsvExport)
//!
//! Enhanced CSV export with funding data.
//!
//! **Stability**: ✅ Stable - Export format is stable
//!
//! ## API Versioning Strategy
//!
//! ### Pre-1.0 Development (Current: 0.1.x)
//!
//! During the pre-1.0 phase:
//! - **Minor versions** (0.1.x → 0.2.0) may include breaking changes
//! - **Patch versions** (0.1.0 → 0.1.1) are backward compatible
//! - Breaking changes will be documented in CHANGELOG.md
//! - Migration guides provided for significant API changes
//!
//! ### Post-1.0 Stability (Future: 1.x.x)
//!
//! After reaching 1.0:
//! - **Major versions** (1.x.x → 2.0.0): Breaking changes allowed
//! - **Minor versions** (1.0.x → 1.1.0): New features, backward compatible
//! - **Patch versions** (1.0.0 → 1.0.1): Bug fixes only
//!
//! ## Stability Classifications
//!
//! ### ✅ Stable
//! - API will not change in backward-incompatible ways within major versions
//! - New methods may be added
//! - Existing methods will not be removed or change signatures
//! - Default values and behavior will remain consistent
//!
//! ### ⚠️ Semi-stable
//! - Core functionality is stable
//! - New variants/options may be added
//! - Existing functionality will not be removed
//! - May have minor behavioral changes with clear migration path
//!
//! ### 🚧 Experimental
//! - API may change significantly
//! - Use with caution in production
//! - Feedback welcome for API design
//! - Will be stabilized in future versions
//!
//! ## Breaking Change Policy
//!
//! ### What Constitutes a Breaking Change
//!
//! 1. **Removing public items** (functions, types, modules)
//! 2. **Changing function signatures** (parameters, return types)
//! 3. **Changing struct fields** (removing, renaming, changing types)
//! 4. **Changing enum variants** (removing, changing data)
//! 5. **Changing default behavior** in ways that affect existing code
//!
//! ### What is NOT a Breaking Change
//!
//! 1. **Adding new public items** (functions, types, modules)
//! 2. **Adding new struct fields** (with default values)
//! 3. **Adding new enum variants** (in non-exhaustive enums)
//! 4. **Adding new trait methods** (with default implementations)
//! 5. **Performance improvements** that don't change behavior
//! 6. **Bug fixes** that correct documented behavior
//!
//! ## Migration Support
//!
//! ### Migration Guides
//!
//! Comprehensive migration guides are provided for:
//! - Upgrading from rs-backtester to hyperliquid-backtester
//! - Major version upgrades within hyperliquid-backtester
//! - Significant API changes during pre-1.0 development
//!
//! See [`migration`](crate::migration) module for detailed guides.
//!
//! ### Deprecation Policy
//!
//! Before removing functionality:
//! 1. **Deprecation warning** added in minor version
//! 2. **Alternative provided** with migration instructions
//! 3. **Minimum one major version** before removal
//! 4. **Clear timeline** communicated in documentation
//!
//! ## Feature Flags and Optional Dependencies
//!
//! ### Current Feature Flags
//!
//! Currently, all features are enabled by default. Future versions may introduce:
//! - `funding-rates` - Enable funding rate calculations (default: enabled)
//! - `csv-export` - Enable CSV export functionality (default: enabled)
//! - `advanced-strategies` - Enable advanced strategy implementations (default: enabled)
//!
//! ### Optional Dependencies
//!
//! Core dependencies are required, but some features may become optional:
//! - CSV export functionality
//! - Advanced mathematical indicators
//! - Visualization helpers
//!
//! ## API Documentation Standards
//!
//! All public API items must include:
//!
//! ### Required Documentation
//! - **Purpose**: What the item does
//! - **Parameters**: Description of all parameters
//! - **Returns**: Description of return value
//! - **Errors**: Possible error conditions
//! - **Examples**: At least one working example
//!
//! ### Optional Documentation
//! - **Performance notes**: For performance-critical items
//! - **Thread safety**: For items used in concurrent contexts
//! - **Stability**: Explicit stability guarantees
//!
//! ## Testing and Quality Assurance
//!
//! ### API Stability Testing
//! - **Compilation tests**: Ensure API changes don't break existing code
//! - **Integration tests**: Test complete workflows
//! - **Documentation tests**: Ensure examples compile and run
//! - **Regression tests**: Prevent API regressions
//!
//! ### Quality Metrics
//! - **Documentation coverage**: >95% for public API
//! - **Test coverage**: >90% for core functionality
//! - **Example coverage**: All public functions have examples
//!
//! ## Community and Feedback
//!
//! ### Feedback Channels
//! - **GitHub Issues**: Bug reports and feature requests
//! - **GitHub Discussions**: API design discussions
//! - **Documentation**: Inline comments and suggestions
//!
//! ### API Design Principles
//! 1. **Ergonomic**: Easy to use correctly, hard to use incorrectly
//! 2. **Consistent**: Similar patterns across the API
//! 3. **Discoverable**: Clear naming and organization
//! 4. **Composable**: Components work well together
//! 5. **Performant**: Efficient by default, with optimization options
//!
//! ## Future Roadmap
//!
//! ### Planned API Additions (Non-breaking)
//! - Additional strategy implementations
//! - Enhanced reporting formats
//! - Performance optimization utilities
//! - Integration with more data sources
//!
//! ### Potential Breaking Changes (Major Version)
//! - Async trait methods for strategies
//! - Enhanced error handling with more context
//! - Streaming data interfaces
//! - Plugin architecture for custom indicators
//!
//! ## Conclusion
//!
//! The hyperliquid-backtester API is designed for long-term stability while allowing
//! for growth and improvement. By following semantic versioning and providing clear
//! migration paths, we aim to minimize disruption while continuously improving the
//! developer experience.
//!
//! For the most up-to-date API documentation, always refer to the latest version
//! on [docs.rs](https://docs.rs/hyperliquid-backtester).