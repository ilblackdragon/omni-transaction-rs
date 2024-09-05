use eyre::Result;
use near_crypto::InMemorySigner;
use near_jsonrpc_client::methods::tx::{RpcTransactionError, TransactionInfo};
use near_jsonrpc_client::{methods, JsonRpcClient};
use near_primitives::hash::CryptoHash;
use near_workspaces::sandbox;
use omni_transaction::near::types::{
    Action, ED25519Signature, Signature as OmniSignature, TransferAction,
};
use omni_transaction::near::utils::PublicKeyStrExt;
use omni_transaction::transaction_builder::{TransactionBuilder, TxBuilder};
use omni_transaction::types::NEAR;
use sha2::Digest;
use tokio::time;

#[tokio::test]
async fn test_send_raw_transaction_created_with_omnitransactionbuilder_for_near() -> Result<()> {
    // Spin up a local Sandbox node.
    let sandbox_worker: near_workspaces::Worker<near_workspaces::network::Sandbox> =
        sandbox().await?;

    // Create two accounts
    let alice = sandbox_worker.dev_create_account().await?;
    let bob = sandbox_worker.dev_create_account().await?;

    let alice_original_balance = alice.view_account().await?.balance;
    let bob_original_balance = bob.view_account().await?.balance;

    let transfer_action = Action::Transfer(TransferAction { deposit: 1u128 });
    let actions = vec![transfer_action];

    // Configure the signer from the first default Sandbox account (Alice).
    let signer = InMemorySigner {
        account_id: alice.id().clone(),
        public_key: alice.secret_key().public_key().to_string().parse()?,
        secret_key: alice.secret_key().to_string().parse()?,
    };

    // Prepare the client
    let rpc_address = sandbox_worker.rpc_addr();
    let rpc_client = JsonRpcClient::connect(rpc_address);

    let latest_block = sandbox_worker.view_block().await?;
    let block_hash = latest_block.hash();
    let block_hash_str = block_hash.to_string();

    let alice_nonce = sandbox_worker
        .view_access_key(alice.id(), &alice.secret_key().public_key())
        .await?
        .nonce
        + 1;

    let alice_public_key = alice.secret_key().public_key().to_string();

    // Build the transaction using OmniTransactionBuilder
    let near_tx = TransactionBuilder::new::<NEAR>()
        .signer_id(alice.id().to_string())
        .signer_public_key(alice_public_key.to_public_key().unwrap())
        .nonce(alice_nonce)
        .receiver_id(bob.id().to_string())
        .block_hash(block_hash_str.to_block_hash().unwrap())
        .actions(actions)
        .build();

    // Build for signing
    let near_tx_encoded = near_tx.build_for_signing();

    // Compute the SHA256 hash of the encoded transaction
    let hashed_tx_value = sha2::Sha256::digest(&near_tx_encoded);

    // Sign the hashed transaction
    let signature = signer.sign(&hashed_tx_value);

    // Create CryptoHash from the SHA256 digest of the signed transaction
    let tx_hash = CryptoHash::hash_bytes(&sha2::Sha256::digest(&near_tx_encoded));

    // @dev For simplicity we only support ED25519 signature in this test
    let signature_bytes: [u8; 64] = match &signature {
        near_crypto::Signature::ED25519(sig) => sig.to_bytes(),
        _ => panic!("Unsupported signature type"),
    };

    let omni_signature = OmniSignature::ED25519(ED25519Signature {
        r: signature_bytes[..32].try_into().unwrap(),
        s: signature_bytes[32..].try_into().unwrap(),
    });

    // Build the signed transaction
    let near_tx_signed = near_tx.build_with_signature(omni_signature);

    let request = methods::send_raw_tx::RpcSendRawTransactionRequest {
        raw_signed_transaction: near_tx_signed.clone(),
        wait_until: near_primitives::views::TxExecutionStatus::IncludedFinal,
    };

    // Send the transaction
    let sent_at = time::Instant::now();

    let response = match rpc_client.call(request).await {
        Ok(response) => response,
        Err(err) => {
            match err.handler_error() {
                Some(RpcTransactionError::TimeoutError) => {}
                _ => Err(err)?,
            }
            loop {
                let response = rpc_client
                    .call(methods::tx::RpcTransactionStatusRequest {
                        transaction_info: TransactionInfo::TransactionId {
                            tx_hash,
                            sender_account_id: signer.account_id.clone(),
                        },
                        wait_until: near_primitives::views::TxExecutionStatus::IncludedFinal,
                    })
                    .await;

                let received_at = time::Instant::now();
                let delta = (received_at - sent_at).as_secs();

                if delta > 60 {
                    Err(eyre::eyre!(
                        "time limit exceeded for the transaction to be recognized"
                    ))?;
                }

                match response {
                    Err(err) => match err.handler_error() {
                        Some(RpcTransactionError::TimeoutError) => {}
                        _ => Err(err)?,
                    },
                    Ok(response) => {
                        break response;
                    }
                }
            }
        }
    };

    assert!(response.final_execution_outcome.is_some());

    let alice_final_balance = alice.view_account().await?.balance;
    let bob_final_balance = bob.view_account().await?.balance;

    let one_yocto_near = 1u128;
    let gas_cost =
        alice_original_balance.as_yoctonear() - alice_final_balance.as_yoctonear() - one_yocto_near;

    let expected_alice_balance = alice_original_balance.as_yoctonear() - gas_cost - one_yocto_near;
    let expected_bob_balance = bob_original_balance.as_yoctonear() + 1;

    assert_eq!(alice_final_balance.as_yoctonear(), expected_alice_balance);
    assert_eq!(bob_final_balance.as_yoctonear(), expected_bob_balance);

    eyre::Ok(())
}
