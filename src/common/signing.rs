use anyhow::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use bincode::{deserialize, serialize};
use serde::Serialize;
use solana_sdk::{
    message::VersionedMessage,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::{Transaction, VersionedTransaction},
};
use solana_trader_proto::api;

use crate::provider::utils::IntoTransactionMessage;

#[derive(Debug, Clone, Serialize)]
pub struct SubmitParams {
    pub skip_pre_flight: bool,
    pub front_running_protection: bool,
    pub use_staked_rpcs: bool,
    pub fast_best_effort: bool,
    pub submit_strategy: api::SubmitStrategy,
    pub allow_bank_run: Option<bool>,
    pub revenue_address: Option<String>,
}

impl Default for SubmitParams {
    fn default() -> Self {
        Self {
            skip_pre_flight: true,
            front_running_protection: false,
            use_staked_rpcs: true,
            fast_best_effort: false,
            submit_strategy: api::SubmitStrategy::PSubmitAll,
            allow_bank_run: None,
            revenue_address: None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SignedTransaction {
    pub content: String,
    pub is_cleanup: bool,
}

pub async fn sign_transaction<T>(
    tx: &T,
    keypair: &Keypair,
    blockhash: String,
) -> Result<SignedTransaction>
where
    T: IntoTransactionMessage + Clone,
{
    let tx_message = tx.clone().into_transaction_message();
    let rawbytes = STANDARD.decode(&tx_message.content)?;
    let parsed_hash = blockhash.parse()?;

    let signed_data = match deserialize(&rawbytes) {
        Ok(versioned_tx) => sign_versioned_transaction(versioned_tx, keypair, parsed_hash)?,
        Err(_) => sign_legacy_transaction(&rawbytes, keypair, parsed_hash)?,
    };

    Ok(SignedTransaction {
        content: STANDARD.encode(signed_data),
        is_cleanup: tx_message.is_cleanup,
    })
}

fn sign_versioned_transaction(
    mut tx: VersionedTransaction,
    keypair: &Keypair,
    blockhash: solana_sdk::hash::Hash,
) -> Result<Vec<u8>> {
    match &mut tx.message {
        VersionedMessage::Legacy(message) => {
            message.recent_blockhash = blockhash;
        }
        VersionedMessage::V0(message) => {
            message.recent_blockhash = blockhash;
        }
    }

    tx.signatures = vec![Signature::default()];
    let message_data = tx.message.serialize();
    tx.signatures[0] = keypair.sign_message(&message_data);

    Ok(serialize(&tx)?)
}

fn sign_legacy_transaction(
    rawbytes: &[u8],
    keypair: &Keypair,
    blockhash: solana_sdk::hash::Hash,
) -> Result<Vec<u8>> {
    let mut tx: Transaction = deserialize(rawbytes)?;
    tx.try_partial_sign(&[keypair], blockhash)?;
    Ok(serialize(&tx)?)
}
