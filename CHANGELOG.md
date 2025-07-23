# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure and core dependencies
- Comprehensive error handling system with `HyperliquidBacktestError`
- Core data structures: `HyperliquidData`, `HyperliquidBacktest`, `HyperliquidCommission`
- Data conversion functionality between Hyperliquid API and rs-backtester formats
- Funding rate utilities and calculations
- Hyperliquid data fetcher with async API integration
- Enhanced commission structure with maker/taker fee distinction
- Enhanced backtesting engine with funding payment calculations
- Strategy extensions including funding arbitrage and enhanced SMA crossover
- Enhanced reporting and metrics with funding-specific analytics
- CSV export functionality with funding rate data
- Comprehensive examples and documentation
- Unit, integration, and performance test suites
- Structured logging and debugging support
- Migration guide from rs-backtester

### Changed
- N/A (initial release)

### Deprecated
- N/A (initial release)

### Removed
- N/A (initial release)

### Fixed
- N/A (initial release)

### Security
- N/A (initial release)

## [0.1.0] - 2024-01-XX

### Added
- Initial release of hyperliquid-backtest
- Core backtesting functionality with Hyperliquid data integration
- Funding rate support and perpetual futures mechanics
- Async data fetching from Hyperliquid API
- Enhanced reporting with funding-specific metrics
- Built-in strategies: funding arbitrage and enhanced SMA crossover
- Comprehensive error handling and type safety
- Structured logging and debugging support
- Migration compatibility with rs-backtester
- Extensive documentation and examples

### Technical Details
- Minimum Rust version: 1.70
- Dependencies: tokio, chrono, serde, thiserror, csv, hyperliquid-rust-sdk, rs-backtester
- API stability: Pre-1.0 development phase with semantic versioning
- Test coverage: >90% with unit, integration, and performance tests
- Documentation: Complete API documentation with examples

### Breaking Changes
- N/A (initial release)

### Migration Guide
- See [Migration Guide](https://docs.rs/hyperliquid-backtest/latest/hyperliquid_backtester/migration/index.html) for upgrading from rs-backtester

---

## Release Process

### Version Numbering
- **Major** (X.0.0): Breaking API changes, significant architectural changes
- **Minor** (0.X.0): New features, backward compatible additions
- **Patch** (0.0.X): Bug fixes, documentation updates, performance improvements

### Pre-1.0 Development
During the pre-1.0 phase (current), minor versions may include breaking changes.
All breaking changes will be clearly documented in this changelog with migration instructions.

### Post-1.0 Stability
After reaching 1.0.0, the public API will be stable within major versions:
- Public items in the `prelude` module are guaranteed stable
- Core data structures maintain backward compatibility
- Error types may add variants but not remove them
- Strategy interfaces remain stable for implementors

### Release Checklist
- [ ] Update version in Cargo.toml
- [ ] Update CHANGELOG.md with release notes
- [ ] Run full test suite: `cargo test --all-features`
- [ ] Run clippy: `cargo clippy -- -D warnings`
- [ ] Run rustfmt: `cargo fmt --check`
- [ ] Update documentation: `cargo doc --no-deps`
- [ ] Test examples: `cargo run --example basic_backtest`
- [ ] Create git tag: `git tag v0.1.0`
- [ ] Publish to crates.io: `cargo publish`
- [ ] Update GitHub release with changelog