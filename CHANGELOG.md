# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/near/omni-transaction-rs/compare/v0.3.2...v0.4.0) - 2025-12-20

### Added

- [**breaking**] Migrate NEAR types to use near-gas-rs and near-token-rs ([#46](https://github.com/near/omni-transaction-rs/pull/46))

### Other

- Enhance README with badges and community links
- Update omni-transaction version in README

## [0.3.2](https://github.com/near/omni-transaction-rs/compare/v0.3.1...v0.3.2) - 2025-12-19

### Other

- Make serde, serde_json, borsh, and schemars optional dependencies (enabled by default until the next breaking change release) ([#42](https://github.com/near/omni-transaction-rs/pull/42))

## [0.3.1](https://github.com/near/omni-transaction-rs/compare/v0.3.0...v0.3.1) - 2025-12-18

### Other

- Removed near-sdk dependency completely, use serde and base64 directly ([#39](https://github.com/near/omni-transaction-rs/pull/39))

## [0.3.0](https://github.com/near/omni-transaction-rs/compare/v0.2.4...v0.3.0) - 2025-11-18

### Added

- [**breaking**] near types will serialize same way as nearcore ([#32](https://github.com/near/omni-transaction-rs/pull/32))

## [0.2.4](https://github.com/near/omni-transaction-rs/compare/v0.2.3...v0.2.4) - 2025-09-04

### Other

- Supported evm transaction string types ([#34](https://github.com/near/omni-transaction-rs/pull/34))

## [0.2.3](https://github.com/near/omni-transaction-rs/compare/v0.2.2...v0.2.3) - 2025-07-19

### Added

- extended actions with Delegate, DeployGlobalContract, UseGlobalContract ([#31](https://github.com/near/omni-transaction-rs/pull/31))

## [0.2.2](https://github.com/near/omni-transaction-rs/compare/v0.2.1...v0.2.2) - 2025-06-23

### Other

- added clone to evm transaction ([#29](https://github.com/near/omni-transaction-rs/pull/29))

## [0.1.1](https://github.com/near/omni-transaction-rs/compare/v0.1.0...v0.1.1) - 2024-12-06

### Other

- Update Cargo.toml
- Update LICENSE-APACHE
