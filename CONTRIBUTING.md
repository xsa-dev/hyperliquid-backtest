# Contributing to Hyperliquid Backtester

Thank you for your interest in contributing to the Hyperliquid Backtester! This document provides guidelines and information for contributors.

## 🚀 Getting Started

### Prerequisites

- **Rust**: 1.70 or later
- **Git**: For version control
- **Internet**: For API testing and dependency downloads

### Development Setup

1. **Fork and Clone**
   ```bash
   git clone https://github.com/xsa-dev/hyperliquid-backtest.git
   cd hyperliquid-backtest
   ```

2. **Build the Project**
   ```bash
   cargo build
   ```

3. **Run Tests**
   ```bash
   cargo test
   ```

4. **Check Code Quality**
   ```bash
   cargo fmt --check
   cargo clippy -- -D warnings
   ```

## 📋 Development Guidelines

### Code Style

We follow standard Rust conventions with some additional guidelines:

- **Formatting**: Use `rustfmt` with default settings
- **Linting**: All `clippy` warnings must be addressed
- **Documentation**: All public APIs must have comprehensive documentation
- **Testing**: New features require corresponding tests

```bash
# Format code
cargo fmt

# Check linting
cargo clippy -- -D warnings

# Generate documentation
cargo doc --no-deps --open
```

### Commit Messages

Use conventional commit format:

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

**Examples:**
```
feat(data): add support for 30s intervals
fix(backtest): correct funding payment calculation
docs(readme): update installation instructions
test(strategies): add funding arbitrage test cases
```

### Branch Naming

Use descriptive branch names:
- `feature/add-new-strategy`
- `fix/funding-calculation-bug`
- `docs/update-examples`
- `refactor/improve-error-handling`

## 🧪 Testing

### Test Categories

1. **Unit Tests**: Test individual functions and methods
2. **Integration Tests**: Test component interactions
3. **Performance Tests**: Benchmark critical operations
4. **API Tests**: Test external API integrations (with mocks)

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test module
cargo test --test integration_tests

# Run with logging
RUST_LOG=debug cargo test

# Run benchmarks
cargo bench

# Test with all features
cargo test --all-features
```

### Writing Tests

- Place unit tests in the same file as the code being tested
- Place integration tests in the `tests/` directory
- Use descriptive test names that explain what is being tested
- Include both positive and negative test cases
- Mock external dependencies (Hyperliquid API calls)

Example test structure:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_funding_calculation_positive_rate() {
        // Test positive funding rate scenario
    }
    
    #[test]
    fn test_funding_calculation_negative_rate() {
        // Test negative funding rate scenario
    }
    
    #[tokio::test]
    async fn test_data_fetching_success() {
        // Test successful data fetching
    }
    
    #[tokio::test]
    async fn test_data_fetching_api_error() {
        // Test API error handling
    }
}
```

## 📚 Documentation

### Documentation Standards

- **Public APIs**: Must have comprehensive rustdoc comments
- **Examples**: Include practical usage examples
- **Error Cases**: Document when functions can fail
- **Safety**: Document any unsafe code (should be rare)

### Documentation Format

```rust
/// Brief description of the function.
///
/// Longer description explaining the purpose, behavior, and any important
/// details about the function.
///
/// # Arguments
///
/// * `param1` - Description of the first parameter
/// * `param2` - Description of the second parameter
///
/// # Returns
///
/// Description of what the function returns.
///
/// # Errors
///
/// This function will return an error if:
/// - Condition 1 occurs
/// - Condition 2 occurs
///
/// # Examples
///
/// ```rust
/// use hyperliquid_backtest::prelude::*;
///
/// let result = function_name(param1, param2)?;
/// assert_eq!(result.value, expected_value);
/// ```
///
/// # Panics
///
/// This function panics if... (only if applicable)
pub fn function_name(param1: Type1, param2: Type2) -> Result<ReturnType, Error> {
    // Implementation
}
```

### Examples

When adding new features, include examples in:
1. **Rustdoc comments**: Simple usage examples
2. **Examples directory**: Complete, runnable examples
3. **README**: Key usage patterns
4. **Integration tests**: Real-world usage scenarios

## 🐛 Bug Reports

### Before Reporting

1. **Search existing issues** to avoid duplicates
2. **Update to latest version** to see if the bug is already fixed
3. **Create minimal reproduction** to isolate the issue

### Bug Report Template

```markdown
## Bug Description
Brief description of the bug.

## Steps to Reproduce
1. Step 1
2. Step 2
3. Step 3

## Expected Behavior
What you expected to happen.

## Actual Behavior
What actually happened.

## Environment
- Rust version: `rustc --version`
- Crate version: `0.1.0`
- Operating system: `uname -a`

## Additional Context
Any additional information, logs, or screenshots.

## Minimal Reproduction
```rust
// Minimal code that reproduces the issue
```

## 💡 Feature Requests

### Feature Request Template

```markdown
## Feature Description
Brief description of the proposed feature.

## Use Case
Describe the problem this feature would solve.

## Proposed Solution
Describe your proposed solution.

## Alternatives Considered
Describe alternative solutions you've considered.

## Additional Context
Any additional information or context.
```

## 🔄 Pull Request Process

### Before Submitting

1. **Create an issue** to discuss the change (for significant features)
2. **Fork the repository** and create a feature branch
3. **Write tests** for your changes
4. **Update documentation** as needed
5. **Run the full test suite**
6. **Check code formatting and linting**

### Pull Request Checklist

- [ ] **Tests**: All tests pass (`cargo test`)
- [ ] **Formatting**: Code is formatted (`cargo fmt`)
- [ ] **Linting**: No clippy warnings (`cargo clippy -- -D warnings`)
- [ ] **Documentation**: Public APIs are documented
- [ ] **Examples**: New features include examples
- [ ] **Changelog**: Update CHANGELOG.md if needed
- [ ] **Breaking Changes**: Clearly marked and documented

### Pull Request Template

```markdown
## Description
Brief description of the changes.

## Type of Change
- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] All tests pass locally

## Documentation
- [ ] Code is documented
- [ ] README updated (if needed)
- [ ] Examples added/updated (if needed)

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] No unnecessary debug prints or commented code
- [ ] Breaking changes are documented
```

## 🏗️ Architecture Guidelines

### Module Organization

```
src/
├── lib.rs              # Public API and re-exports
├── data.rs             # Data structures and fetching
├── backtest.rs         # Backtesting engine
├── strategies.rs       # Trading strategies
├── indicators.rs       # Technical indicators
├── errors.rs           # Error types
├── funding_report.rs   # Funding-specific reporting
├── csv_export.rs       # Data export functionality
├── migration.rs        # Migration utilities
├── api_docs.rs         # API documentation
└── tests/              # Integration tests
```

### Design Principles

1. **Async First**: Use async/await for I/O operations
2. **Error Handling**: Comprehensive error types with context
3. **Type Safety**: Leverage Rust's type system for correctness
4. **Performance**: Optimize for large datasets
5. **Compatibility**: Maintain compatibility with rs-backtester
6. **Extensibility**: Design for easy extension and customization

### API Design

- **Consistent Naming**: Use clear, consistent naming conventions
- **Builder Pattern**: For complex configuration objects
- **Result Types**: All fallible operations return `Result<T, E>`
- **Prelude Module**: Export commonly used types
- **Semantic Versioning**: Follow SemVer for API changes

## 🚀 Release Process

### Version Bumping

1. **Patch** (0.1.0 → 0.1.1): Bug fixes, documentation updates
2. **Minor** (0.1.0 → 0.2.0): New features, backward compatible
3. **Major** (0.1.0 → 1.0.0): Breaking changes

### Release Checklist

- [ ] Update version in `Cargo.toml`
- [ ] Update `CHANGELOG.md`
- [ ] Run full test suite
- [ ] Update documentation
- [ ] Create git tag
- [ ] Publish to crates.io
- [ ] Create GitHub release

## 📞 Getting Help

- **Issues**: [GitHub Issues](https://github.com/xsa-dev/hyperliquid-backtest/issues)
- **Discussions**: [GitHub Discussions](https://github.com/xsa-dev/hyperliquid-backtest/discussions)
- **Documentation**: [docs.rs](https://docs.rs/hyperliquid-backtest)

## 📄 License

By contributing to this project, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

## 🙏 Recognition

Contributors will be recognized in:
- **README.md**: Contributors section
- **CHANGELOG.md**: Release notes
- **GitHub**: Contributor graphs and statistics

Thank you for contributing to Hyperliquid Backtester! 🚀