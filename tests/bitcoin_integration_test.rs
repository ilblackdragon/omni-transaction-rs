// Bitcoin integration tests have been temporarily disabled to remove the
// dependency on omni-testing-utilities, which pulls in near-sdk.
//
// These tests are valuable but rely on external test utilities for:
// - Bitcoin testnet setup
// - Test account management
// - UTXO handling
//
// To re-enable these tests, either:
// 1. Vendor the necessary test utilities from omni-testing-utilities
// 2. Rewrite the tests using alternative Bitcoin test frameworks
// 3. Use the bitcoind test utilities directly
//
// The original tests can be found in bitcoin_integration_test.rs.disabled
