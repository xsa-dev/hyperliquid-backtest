# Publishing Guide for Hyperliquid Backtester

This document outlines the steps and requirements for publishing the hyperliquid-backtester crate to crates.io.

## Pre-Publication Checklist

### Code Quality
- [ ] All compilation errors resolved
- [ ] All tests passing (`cargo test`)
- [ ] No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Documentation builds without warnings (`cargo doc`)

### Documentation
- [ ] README.md is comprehensive and up-to-date
- [ ] All public APIs have rustdoc comments
- [ ] Examples are working and well-documented
- [ ] CHANGELOG.md is updated
- [ ] CONTRIBUTING.md provides clear guidelines

### Package Configuration
- [ ] Cargo.toml metadata is complete
- [ ] License files are present (MIT and Apache-2.0)
- [ ] Keywords and categories are appropriate
- [ ] Version number follows SemVer
- [ ] Dependencies are properly specified

### Testing
- [ ] Unit tests cover core functionality
- [ ] Integration tests work with real API (mocked)
- [ ] Examples run successfully
- [ ] Performance benchmarks are reasonable

## Publication Steps

### 1. Final Preparation

```bash
# Ensure all tests pass
cargo test --all-features

# Check for any issues
cargo clippy -- -D warnings

# Format code
cargo fmt

# Build documentation
cargo doc --no-deps

# Test examples
cargo run --example getting_started
cargo run --example basic_backtest
```

### 2. Version Management

Update version in `Cargo.toml`:
```toml
[package]
version = "0.1.0"  # Follow SemVer
```

Update `CHANGELOG.md` with release notes.

### 3. Package Verification

```bash
# Create package
cargo package

# Verify package contents
cargo package --list

# Test the packaged version
cargo package --allow-dirty
cd target/package/hyperliquid-backtester-0.1.0
cargo test
```

### 4. Publish to crates.io

```bash
# Login to crates.io (one-time setup)
cargo login

# Publish (dry run first)
cargo publish --dry-run

# Actual publish
cargo publish
```

### 5. Post-Publication

- Create GitHub release with changelog
- Update documentation links
- Announce on relevant forums/communities
- Monitor for issues and feedback

## Package Metadata

The crate is configured with the following metadata:

```toml
[package]
name = "hyperliquid-backtester"
version = "0.1.0"
edition = "2021"
rust-version = "1.70"
authors = ["Hyperliquid Backtester Contributors"]
description = "Comprehensive Rust library for backtesting trading strategies with Hyperliquid data, funding rates, and perpetual futures mechanics"
documentation = "https://docs.rs/hyperliquid-backtester"
homepage = "https://github.com/xsa-dev/hyperliquid-backtester"
repository = "https://github.com/xsa-dev/hyperliquid-backtester"
license = "MIT OR Apache-2.0"
keywords = ["trading", "backtesting", "hyperliquid", "cryptocurrency", "defi"]
categories = ["finance", "api-bindings", "algorithms", "simulation", "mathematics"]
```

## API Stability

### Current Version (0.1.0)
- Pre-1.0 development phase
- Public API may change between minor versions
- Breaking changes documented in CHANGELOG.md
- Migration guides provided for significant changes

### Future Stability (1.0.0+)
- Public API in prelude module stable within major versions
- Core data structures stable
- Error types may add variants but not remove them
- Strategy trait interfaces stable for implementors

## Dependencies

### Runtime Dependencies
- `tokio`: Async runtime
- `chrono`: Date/time handling
- `serde`: Serialization
- `thiserror`: Error handling
- `csv`: Data export
- `log`, `tracing`, `tracing-subscriber`: Logging
- `reqwest`: HTTP client
- `hyperliquid_rust_sdk`: Hyperliquid API (path dependency)
- `rs-backtester`: Backtesting framework (path dependency)

### Development Dependencies
- `tokio-test`: Async testing
- `criterion`: Benchmarking
- `mockito`, `wiremock`: API mocking
- `proptest`: Property-based testing
- `memory-stats`, `sysinfo`: Performance monitoring
- `tempfile`: Temporary files for tests

## Known Issues

Before publication, the following issues need to be resolved:

1. **Compilation Errors**: Several compilation errors need fixing
2. **API Compatibility**: Ensure compatibility with latest versions of dependencies
3. **Test Coverage**: Complete test suite implementation
4. **Documentation**: Ensure all examples compile and run

## Support and Maintenance

### Issue Tracking
- GitHub Issues for bug reports
- GitHub Discussions for questions
- Clear issue templates and labels

### Release Schedule
- Patch releases: Bug fixes, documentation updates
- Minor releases: New features, backward compatible
- Major releases: Breaking changes (post-1.0)

### Community
- Contributing guidelines in CONTRIBUTING.md
- Code of conduct
- Recognition for contributors

## Security

### Vulnerability Reporting
- Security issues should be reported privately
- Contact information in README.md
- Responsible disclosure policy

### Dependencies
- Regular dependency updates
- Security audit of dependencies
- Minimal dependency footprint

## Performance

### Benchmarks
- Performance benchmarks in `benches/`
- CI integration for performance regression detection
- Memory usage monitoring

### Optimization
- Async/await for I/O operations
- Efficient data structures
- Streaming for large datasets
- Caching where appropriate

## Documentation

### API Documentation
- Comprehensive rustdoc comments
- Examples in documentation
- Error handling guidance
- Migration guides

### User Documentation
- README with quick start
- Comprehensive examples
- Best practices guide
- Troubleshooting section

### Developer Documentation
- Contributing guidelines
- Architecture overview
- Testing strategy
- Release process

This publishing guide ensures the crate meets high standards for quality, documentation, and maintainability before publication to crates.io.