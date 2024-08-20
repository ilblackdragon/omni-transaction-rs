use alloy::providers::Provider;
use alloy::signers::Signer;
use alloy::{
    network::EthereumWallet, node_bindings::Anvil, providers::ProviderBuilder,
    signers::local::PrivateKeySigner,
};
use alloy_primitives::{keccak256, U256};
use eyre::Result;
use std::result::Result::Ok;

use omni_transaction::evm::utils::parse_eth_address;
use omni_transaction::transaction_builder::{
    TransactionBuilder as OmniTransactionBuilder, TxBuilder,
};
use omni_transaction::types::{Signature as OmniSignature, EVM};

const MAX_FEE_PER_GAS: u128 = 20_000_000_000;
const MAX_PRIORITY_FEE_PER_GAS: u128 = 1_000_000_000;
const GAS_LIMIT: u128 = 21_000;

#[tokio::test]
async fn test_send_raw_transaction_created_with_omnitransactionbuilder() -> Result<()> {
    let nonce: u64 = 0;
    let to_address_str = "d8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
    let to_address = parse_eth_address(to_address_str);
    let value_as_128 = 10000000000000000u128; // 0.01 ETH
    let value = U256::from(value_as_128);
    let data: Vec<u8> = vec![];

    // Spin up a local Anvil node.
    let anvil = Anvil::new().block_time(1).try_spawn()?;

    // Configure the signer from the first default Anvil account (Alice).
    let signer: PrivateKeySigner = anvil.keys()[0].clone().into();
    let wallet = EthereumWallet::from(signer.clone());

    // Create a provider with the wallet.
    let rpc_url = anvil.endpoint().parse()?;
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet.clone())
        .on_http(rpc_url);

    let signer_balance = provider.get_balance(signer.address()).await?;

    assert!(signer_balance >= value);

    // ========= OmniTransactionBuilder tx hash and signature =========

    // Build the transaction using OmniTransactionBuilder
    let omni_evm_tx = OmniTransactionBuilder::new::<EVM>()
        .nonce(nonce)
        .to(to_address)
        .value(value_as_128)
        .input(data.clone())
        .max_priority_fee_per_gas(MAX_PRIORITY_FEE_PER_GAS)
        .max_fee_per_gas(MAX_FEE_PER_GAS)
        .gas_limit(GAS_LIMIT)
        .chain_id(anvil.chain_id())
        .build();

    // Encode the transaction with EIP-1559 prefix
    let omni_evm_tx_encoded = omni_evm_tx.build_for_signing();

    // Hash the encoded transaction
    let omni_evm_tx_hash = keccak256(&omni_evm_tx_encoded);

    // Sign the transaction hash
    let signature = signer.sign_hash(&omni_evm_tx_hash).await?;

    let signature_omni: OmniSignature = OmniSignature {
        v: signature.v().to_u64(),
        r: signature.r().to_be_bytes::<32>().to_vec(),
        s: signature.s().to_be_bytes::<32>().to_vec(),
    };

    let omni_evm_tx_encoded_with_signature = omni_evm_tx.build_with_signature(&signature_omni);

    // Send the transaction
    match provider
        .send_raw_transaction(&omni_evm_tx_encoded_with_signature)
        .await
    {
        Ok(tx_hash) => println!("Transaction sent successfully. Hash: {:?}", tx_hash),
        Err(e) => println!("Failed to send transaction: {:?}", e),
    }

    eyre::Ok(())
}
