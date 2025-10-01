# Contributing to Vagus

Thank you for your interest in contributing to Vagus! This document provides guidelines and information for contributors.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Contributing Process](#contributing-process)
- [Coding Standards](#coding-standards)
- [Testing Guidelines](#testing-guidelines)
- [Commit Guidelines](#commit-guidelines)
- [Pull Request Process](#pull-request-process)
- [Issue Reporting](#issue-reporting)

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/your-username/vagus.git
   cd vagus
   ```
3. **Add the upstream remote**:
   ```bash
   git remote add upstream https://github.com/vagus-io/vagus.git
   ```

## Development Setup

### Prerequisites

- **Rust** (latest stable) - for gateway, oracle, and relayer services
- **Foundry** - for Solidity development
- **Python 3.11+** - for planner tooling
- **Docker** - for cross-chain development
- **Node.js** - for development tooling

### Environment Setup

1. **Start the development environment**:
   ```bash
   # Start EVM + Cosmos chains, gateways, oracle, and relayers
   ./infra/devnet/up.sh
   ```

2. **Deploy contracts**:
   ```bash
   # EVM contracts
   cd contracts
   forge script script/DeployCore.s.sol --rpc-url http://localhost:8545 --broadcast
   ```

3. **Install Python dependencies**:
   ```bash
   cd planner
   pip install -e .[dev]
   ```

## Contributing Process

### 1. Choose an Issue

- Look for issues labeled `good first issue` or `help wanted`
- Comment on the issue to indicate you're working on it
- For larger features, discuss the approach in the issue first

### 2. Create a Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/issue-number-description
```

### 3. Make Your Changes

Follow the coding standards and testing guidelines outlined below.

### 4. Test Your Changes

Run the appropriate test suites:

```bash
# Solidity tests
cd contracts
forge test -vv

# Rust tests
cargo test --workspace

# Python tests
cd planner
pytest

# Cross-chain integration tests
cd tests/golden
cargo run -- run-all --evm-rpc http://localhost:8545 --cosmos-rpc http://localhost:26657
```

### 5. Commit Your Changes

Follow the [commit guidelines](#commit-guidelines) below.

### 6. Push and Create a Pull Request

```bash
git push origin feature/your-feature-name
```

Then create a pull request on GitHub.

## Coding Standards

### Solidity

- **Version**: ^0.8.24
- **Style**: Use `forge fmt` to format code
- **Naming**: PascalCase for contracts, I-prefix for interfaces
- **Documentation**: All external functions must have NatSpec comments
- **Security**: Use custom errors over require strings where reasonable

### Rust

- **Edition**: 2021
- **Style**: Use `cargo fmt` and `cargo clippy --all-targets --all-features`
- **Naming**: snake_case for modules, CamelCase for types
- **Dependencies**: Prefer tokio, anyhow, thiserror, tracing
- **EVM Bindings**: Use ethers-rs or alloy with typed Abigen

### Python

- **Version**: 3.11+
- **Style**: Use `ruff check`, `black .`, and `mypy --strict`
- **Naming**: snake_case for modules and functions
- **Dependencies**: Use pydantic for schemas

### YAML Schemas

- Must specify units and bounds
- Mark "brakeable" fields explicitly
- Provide codegen step for "scaledLimitsHash"

## Testing Guidelines

### Solidity Tests

- Use Foundry with 256 fuzz runs by default
- Name test cases descriptively (`testRevertsWhen...`)
- Include invariant tests for ANS state machine hysteresis
- Test capability revocation semantics

### Rust Tests

- Use `cargo test --workspace` for all tests
- Focused testing: `cargo test -p vagus-gateway telemetry::`
- Include integration tests against anvil devnet
- Use feature flags: `sim` for simulated sensors, `hw` for hardware

### Python Tests

- Use pytest with fixtures under `planner/tests/fixtures/`
- Document new CBOR vectors via `planner/generate_cbor_vectors.py`
- Test EIP-712 signature generation and validation

### Cross-Chain Tests

- Use the golden test suite for multichain invariants
- Test both EVM and CosmWasm implementations
- Verify event consistency across chains

## Commit Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `style`: Code style changes (formatting, etc.)
- `refactor`: Code refactoring
- `test`: Adding or updating tests
- `chore`: Maintenance tasks

### Scopes

- `contracts`: Solidity contract changes
- `gateway`: Rust gateway service changes
- `oracle`: Rust oracle service changes
- `planner`: Python planner tooling changes
- `schemas`: YAML schema and policy changes
- `docs`: Documentation changes
- `infra`: Infrastructure and deployment changes

### Examples

```
feat(contracts): add VagalBrake contract with ANS state scaling
fix(gateway): resolve telemetry collection race condition
docs: update architecture diagram for cross-chain flow
```

## Pull Request Process

### Before Submitting

1. **Ensure all tests pass** locally
2. **Update documentation** if needed
3. **Add tests** for new functionality
4. **Check for breaking changes** and document them
5. **Run linting** and fix any issues

### PR Description Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests pass
- [ ] Integration tests pass
- [ ] Cross-chain tests pass (if applicable)

## Checklist
- [ ] Code follows project style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Breaking changes documented
```

### Review Process

1. **Automated checks** must pass (CI/CD)
2. **Code review** by maintainers
3. **Testing** in development environment
4. **Approval** from at least one maintainer

## Issue Reporting

### Bug Reports

Use the bug report template and include:

- **Description**: Clear description of the bug
- **Steps to Reproduce**: Detailed steps to reproduce
- **Expected Behavior**: What should happen
- **Actual Behavior**: What actually happens
- **Environment**: OS, versions, etc.
- **Logs**: Relevant log output

### Feature Requests

Use the feature request template and include:

- **Description**: Clear description of the feature
- **Use Case**: Why this feature is needed
- **Proposed Solution**: How you envision it working
- **Alternatives**: Other solutions considered

### Security Issues

**Do not** report security vulnerabilities through public issues. Instead, see [SECURITY.md](SECURITY.md) for reporting procedures.

## Getting Help

- **Documentation**: Check the `docs/` directory
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Use GitHub Discussions for questions
- **Community**: Join our community channels (to be announced)

## Recognition

Contributors will be recognized in:

- CONTRIBUTORS.md file
- Release notes
- Project documentation
- Community acknowledgments

Thank you for contributing to Vagus! 🚀
