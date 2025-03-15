//! Commonly used functionality related to the `rpc_client`.

use anyhow::{Context as _, Result};
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::config::RpcSendTransactionConfig;
use solana_sdk::{
    commitment_config::CommitmentConfig, instruction::Instruction, signature::Signature,
    signer::signers::Signers, transaction::Transaction,
};

pub trait RpcClientExt {
    async fn send_with_payer_latest_blockhash_with_spinner<SigningKeyparis: Signers + ?Sized>(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &SigningKeyparis,
    ) -> Result<Signature>;

    async fn send_with_payer_latest_blockhash_with_spinner_and_config<
        SigningKeyparis: Signers + ?Sized,
    >(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &SigningKeyparis,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature>;
}

impl RpcClientExt for RpcClient {
    async fn send_with_payer_latest_blockhash_with_spinner<SigningKeyparis: Signers + ?Sized>(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &SigningKeyparis,
    ) -> Result<Signature> {
        self.send_with_payer_latest_blockhash_with_spinner_and_config(
            instructions,
            payer,
            signing_keypairs,
            RpcSendTransactionConfig::default(),
        )
        .await
    }

    async fn send_with_payer_latest_blockhash_with_spinner_and_config<
        SigningKeyparis: Signers + ?Sized,
    >(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &SigningKeyparis,
        config: RpcSendTransactionConfig,
    ) -> Result<Signature> {
        // When the RpcClient is configured with a commitment that is not Finalized, I often see
        // "Blockhash not found" errors.  Considering that we run transactions right away, there
        // seems to be no downside in using slightly older block hashes as reference points.
        let (latest_blockhash, _) = self
            .get_latest_blockhash_with_commitment(CommitmentConfig::finalized())
            .await
            .context("Getting a blockhash from the cluster")?;

        let transaction = Transaction::new_signed_with_payer(
            instructions,
            payer,
            signing_keypairs,
            latest_blockhash,
        );

        self.send_and_confirm_transaction_with_spinner_and_config(
            &transaction,
            self.commitment(),
            config,
        )
        .await
        .context("Transaction execution failed")
    }
}
