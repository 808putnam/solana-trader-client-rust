# Solana Trader Rust SDK

## Objective

This SDK is designed to make it easy for you to use the bloXroute Labs API in Rust. 

## Installation

``cargo add solana-trader-client-rust``

or

```toml
[dependencies]
solana-trader-client-rust = "0.1.0"
```

## Usage

The SDK provides access to Solana Trader API through:

- gRPC: High-performance RPC calls
- HTTP: Simple REST requests
- WebSocket: Real-time streaming data


### Client Initialization

Refer to **SETUP.md** for available networks, regions IDE setup and notes on testing.

Create and populate your `.env` file with something like this:

```bash
PUBLIC_KEY="...."
PRIVATE_KEY="......."
AUTH_HEADER="......"
NETWORK=MAINNET
REGION=NY
```

A simple example:

```rust
let request = api::GetRaydiumQuotesRequest {
    in_token: WRAPPED_SOL.to_string(),
    out_token: USDC.to_string(), 
    in_amount: 0.1,
    slippage: 0.2,
};

// Using GRPC
let response = grpc_client.get_raydium_quotes(&request).await?;

// Using HTTP
let response = http_client.get_raydium_quotes(&request).await?;

// Using WebSocket
let response = ws_client.get_raydium_quotes(&request).await?;
```

Please refer to the `tests` directory for more examples.

## Publishing
### Setup

We use the `bx-circle-ci` credentials for managing our [crates.io](crates.io) account. To login to [crates.io](crates.io), logout of your github account and log back in to github with the `bx-circle-ci` credentials from `1Password`. You can then navigate to [crates.io](crates.io) and select `login with Github`.

The [crates.io](crates.io) site is where we manage API tokens for publishing our rust crates. You can navigate to the `bx-circle-ci` credentials page in `1Password` to get the latest [crates.io](crates.io) token.

### Steps

1. Start with a clean pull of the repo you would like to publish.
    1. solana-trader-proto
    2. solana-trader-client-rust
2. Run `cargo publish --dry-run` to see if you will have any issues publishing.
    1. If you get an error mentioning files have not been checked into git and suggesting to use `--allow-dirty`, be sure to track down the folder/files that are causing the error and resolve before proceeding.
3. If `cargo publish --dry-run` is successful, run `cargo publish --token <api token>`.