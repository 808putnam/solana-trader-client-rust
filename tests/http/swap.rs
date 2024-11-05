use anyhow::Result;
use solana_trader_client_rust::common::{constants::USDC, constants::WRAPPED_SOL};
use solana_trader_client_rust::provider::http::HTTPClient;
use solana_trader_proto::api;
use solana_trader_proto::api::TransactionMessage;
use test_case::test_case;

#[test_case(
    WRAPPED_SOL,
    USDC,
    0.001,
    20.0;
    "Raydium SOL to USDC swap via HTTP"
)]
#[tokio::test]
#[ignore]
async fn test_raydium_swap_http(
    in_token: &str,
    out_token: &str,
    in_amount: f64,
    slippage: f64,
) -> Result<()> {
    let client = HTTPClient::new(None)?;

    let request = api::PostRaydiumSwapRequest {
        owner_address: client
            .public_key
            .unwrap_or_else(|| panic!("Public key is required for Raydium swap"))
            .to_string(),
        in_token: in_token.to_string(),
        out_token: out_token.to_string(),
        in_amount,
        slippage,
        compute_limit: 300000,
        compute_price: 2000,
        tip: Some(2000001),
    };

    let response = client.post_raydium_swap(&request).await?;
    println!(
        "Raydium Quote: {}",
        serde_json::to_string_pretty(&response)?
    );

    let txs = response.transactions.as_slice();
    for tx in txs {
        let s = client
            .sign_and_submit(
                TransactionMessage {
                    content: tx.clone().content,
                    is_cleanup: tx.is_cleanup,
                },
                true,
                false,
                false,
                false,
            )
            .await;
        println!("Raydium signature: {}", s?);
    }

    Ok(())
}

#[test_case(
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "So11111111111111111111111111111111111111112",
    0.01,
    0.5;
    "Raydium CPMM USDC to SOL swap via HTTP"
)]
#[tokio::test]
#[ignore]
async fn test_raydium_cpmm_swap_http(
    in_token: &str,
    out_token: &str,
    in_amount: f64,
    slippage: f64,
) -> Result<()> {
    let client = HTTPClient::new(None)?;

    let request = api::PostRaydiumCpmmSwapRequest {
        owner_address: client
            .public_key
            .unwrap_or_else(|| panic!("Public key is required for Raydium CPMM swap"))
            .to_string(),
        pool_address: "".to_string(),
        in_token: in_token.to_string(),
        out_token: out_token.to_string(),
        in_amount,
        slippage,
        compute_limit: 300000,
        compute_price: 2000,
        tip: Some(2000001),
    };

    let response = client.post_raydium_cpmm_swap(&request).await?;
    println!(
        "Raydium CPMM Quote: {}",
        serde_json::to_string_pretty(&response)?
    );

    let txs = response.transaction.as_slice();
    for tx in txs {
        let s = client
            .sign_and_submit(
                TransactionMessage {
                    content: tx.clone().content,
                    is_cleanup: tx.is_cleanup,
                },
                true,
                false,
                false,
                false,
            )
            .await;
        println!("Raydium CPMM signature: {}", s?);
    }

    Ok(())
}

#[test_case(
    "So11111111111111111111111111111111111111112",   // SOL
    "HDa3zJc12ahykSsBRvgiWzr6WLEByf36yzKKbVvy4gnF", // USDC
    0.0089,
    0.1;
    "Raydium CLMM SOL to USDC swap via HTTP"
)]
#[tokio::test]
#[ignore]
async fn test_raydium_clmm_swap_http(
    in_token: &str,
    out_token: &str,
    in_amount: f64,
    slippage: f64,
) -> Result<()> {
    let client = HTTPClient::new(None)?;

    let request = api::PostRaydiumSwapRequest {
        owner_address: client
            .public_key
            .unwrap_or_else(|| panic!("Public key is required for Raydium CLMM swap"))
            .to_string(),
        in_token: in_token.to_string(),
        out_token: out_token.to_string(),
        in_amount,
        slippage,
        compute_limit: 300000,
        compute_price: 10000,
        tip: Some(10000),
    };

    let response = client.post_raydium_clmm_swap(&request).await?;
    println!(
        "Raydium CLMM Quote: {}",
        serde_json::to_string_pretty(&response)?
    );

    let txs = response.transactions.as_slice();
    for tx in txs {
        let s = client
            .sign_and_submit(
                TransactionMessage {
                    content: tx.clone().content,
                    is_cleanup: tx.is_cleanup,
                },
                true,
                false,
                false,
                false,
            )
            .await;
        println!("Raydium CLMM signature: {}", s?);
    }

    Ok(())
}

#[test_case(
    WRAPPED_SOL,
    USDC,
    0.01,
    1.0;
    "Jupiter SOL to USDC swap via HTTP"
)]
#[tokio::test]
#[ignore]
async fn test_jupiter_swap_http(
    in_token: &str,
    out_token: &str,
    in_amount: f64,
    slippage: f64,
) -> Result<()> {
    let client = HTTPClient::new(None)?;

    let request = api::PostJupiterSwapRequest {
        owner_address: client
            .public_key
            .unwrap_or_else(|| panic!("Public key is required for Jupiter swap"))
            .to_string(),
        in_token: in_token.to_string(),
        out_token: out_token.to_string(),
        in_amount,
        slippage,
        compute_limit: 300000,
        compute_price: 2000,
        tip: Some(2000001),
        fast_mode: None,
    };

    let response = client.post_jupiter_swap(&request).await?;
    println!(
        "Jupiter Quote: {}",
        serde_json::to_string_pretty(&response)?
    );

    let txs = response.transactions.as_slice();
    for tx in txs {
        let s = client
            .sign_and_submit(
                TransactionMessage {
                    content: tx.clone().content,
                    is_cleanup: tx.is_cleanup,
                },
                true,
                false,
                false,
                false,
            )
            .await;
        println!("Jupiter signature: {}", s?);
    }

    Ok(())
}
