use base64::engine::general_purpose;
use base64::Engine;
use dotenv::dotenv;
use solana_hash::Hash;
use solana_sdk::instruction::{AccountMeta, CompiledInstruction, Instruction};
use solana_sdk::message::VersionedMessage;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Signature, Signer};
use solana_sdk::system_instruction;
use solana_sdk::transaction::{Transaction, VersionedTransaction};
use solana_trader_client_rust::common::signing::SubmitParams;
use solana_trader_client_rust::provider::grpc::GrpcClient;
use solana_trader_proto::api::{GetRecentBlockHashRequestV2, TransactionMessage};
use std::str::FromStr;

#[tokio::test]
#[ignore]
async fn test_add_memo_to_tx() -> anyhow::Result<()> {
    let mut client = GrpcClient::new(None).await?;

    let lamports_to_transfer = 1000000;
    let keypair = client.keypair.as_ref().unwrap();
    let pubkey = keypair.pubkey();

    // a simple transfer that acts as our main instruction
    let transfer_to_self_instruction = system_instruction::transfer(
        &pubkey.clone(),
        &client.public_key.unwrap(),
        lamports_to_transfer,
    );

    let jito_tip_wallet = Pubkey::from_str("95cfoy472fcQHaw4tPGBTKpn6ZQnfEPfBgDQx6gcRmRg").unwrap();

    let jito_tip_instruction =
        system_instruction::transfer(&pubkey.clone(), &jito_tip_wallet, lamports_to_transfer);
    let block_hash = client
        .get_recent_block_hash_v2(GetRecentBlockHashRequestV2 { offset: 0 })
        .await;
    let keypair = client.keypair.as_ref().unwrap();
    let mut transaction = Transaction::new_signed_with_payer(
        &[
            transfer_to_self_instruction,
            jito_tip_instruction,
            build_memo_instruction(),
        ],
        Some(&client.public_key.unwrap()),
        &[&keypair],
        Hash::from_str(&*block_hash?.block_hash).unwrap(),
    );

    // we have a simple transaction now, we sign it and serialize it
    transaction.signatures = vec![Signature::default(); 1];
    let message_data = transaction.message.serialize().to_vec();
    let signature = keypair.sign_message(&message_data);
    transaction.signatures[0] = signature;
    let serialized_tx = bincode::serialize(&transaction);

    let vec: Vec<TransactionMessage> = vec![TransactionMessage {
        content: general_purpose::STANDARD.encode(serialized_tx?),
        is_cleanup: false,
    }];

    let submit_opts = SubmitParams::default();
    let s = client.sign_and_submit(vec, submit_opts, false).await;
    println!("signature2 : {:#?}", s?);
    Ok(())
}

#[tokio::test]
#[ignore]
async fn test_add_memo_to_serialized_tx() -> anyhow::Result<()> {
    dotenv().ok();

    let mut client = GrpcClient::new(None).await?;

    let lamports_to_transfer = 2000;
    let keypair = client.keypair.as_ref().unwrap();
    let pubkey = keypair.pubkey();
    let transfer_instruction = system_instruction::transfer(
        &pubkey.clone(),
        &client.public_key.unwrap(),
        lamports_to_transfer,
    );

    let mut transaction =
        Transaction::new_with_payer(&[transfer_instruction], Some(&pubkey.clone()));

    // we have a simple transaction now, we sign it and serialize it
    transaction.signatures = vec![Signature::default(); 1];
    let message_data = transaction.message.serialize().to_vec();
    let signature = keypair.sign_message(&message_data);
    transaction.signatures[0] = signature;
    let serialized_tx = bincode::serialize(&transaction);

    // we deserialize the tx and add our memo to it , sign it and serialized it again
    let mut deserialized_tx =
        bincode::deserialize::<VersionedTransaction>(&serialized_tx.unwrap()).unwrap();

    let msg = &mut deserialized_tx.message;

    let trader_apimemo_program =
        Pubkey::from_str("HQ2UUt18uJqKaQFJhgV9zaTdQxUZjNrsKFgoEDquBkcx").unwrap();
    let mut account_keys = msg.static_account_keys().to_vec();
    account_keys.push(trader_apimemo_program);

    if let VersionedMessage::Legacy(message) = msg {
        let bx_memo_marker_msg = String::from("Powered by bloXroute Trader Api")
            .as_bytes()
            .to_vec();

        message.account_keys = account_keys.clone();
        message
            .instructions
            .push(CompiledInstruction::new_from_raw_parts(
                (message.clone().account_keys.len() - 1) as u8,
                bx_memo_marker_msg,
                vec![],
            ));
    }

    let content = bincode::serialize(&deserialized_tx)?;

    let vec: Vec<TransactionMessage> = vec![TransactionMessage {
        content: general_purpose::STANDARD.encode(content),
        is_cleanup: false,
    }];
    let submit_opts = SubmitParams {
        use_staked_rpcs: false,
        ..Default::default()
    };
    let s = client.sign_and_submit(vec, submit_opts, false).await;
    println!("signature : {:#?}", s?);
    Ok(())
}

fn build_memo_instruction() -> Instruction {
    let trader_apimemo_program =
        Pubkey::from_str("HQ2UUt18uJqKaQFJhgV9zaTdQxUZjNrsKFgoEDquBkcx").unwrap();

    let accounts = vec![AccountMeta::new(trader_apimemo_program, false)];
    let bx_memo_marker_msg = String::from("Powered by bloXroute Trader Api")
        .as_bytes()
        .to_vec();

    Instruction {
        program_id: trader_apimemo_program,
        accounts,
        data: bx_memo_marker_msg,
    }
}
