//! Commonly used functionality related to the `rpc_client`.

use anyhow::{Context as _, Result};
use solana_program::pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, signature::Signature, signer::signers::Signers,
    transaction::Transaction,
};

pub trait RpcClientExt {
    async fn send_with_payer_latest_blockhash_with_spinner<SigningKeyparis: Signers + ?Sized>(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &SigningKeyparis,
    ) -> Result<Signature>;
}

impl RpcClientExt for RpcClient {
    async fn send_with_payer_latest_blockhash_with_spinner<SigningKeyparis: Signers + ?Sized>(
        &self,
        instructions: &[Instruction],
        payer: Option<&Pubkey>,
        signing_keypairs: &SigningKeyparis,
    ) -> Result<Signature> {
        let latest_blockhash = self
            .get_latest_blockhash()
            .await
            .context("Getting a blockhash from the cluster")?;

        let transaction = Transaction::new_signed_with_payer(
            instructions,
            payer,
            signing_keypairs,
            latest_blockhash,
        );

        self.send_and_confirm_transaction_with_spinner(&transaction)
            .await
            .context("Transaction execution failed")
    }
}
