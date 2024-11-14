pub mod constants;
pub mod signing;

use std::{env, str::FromStr};

use anyhow::{anyhow, Result};
use constants::{LOCAL, MAINNET_NY, MAINNET_PUMP_NY, MAINNET_PUMP_UK, MAINNET_UK, TESTNET};
use dotenv::dotenv;
use solana_sdk::{bs58::decode, pubkey::Pubkey, signature::Keypair};

pub fn http_endpoint(base_url: &str, secure: bool) -> String {
    let prefix = if secure { "https" } else { "http" };
    format!("{}://{}", prefix, base_url)
}

pub fn ws_endpoint(base_url: &str, secure: bool) -> String {
    let prefix = if secure { "wss" } else { "ws" };
    format!("{}://{}/ws", prefix, base_url)
}

pub fn grpc_endpoint(base_url: &str, secure: bool) -> String {
    let prefix = if secure { "https" } else { "http" };
    let port = if secure { ":443" } else { "" };
    format!("{}://{}{}", prefix, base_url, port)
}

pub fn get_base_url_from_env() -> (String, bool) {
    let network = std::env::var("NETWORK").unwrap_or_else(|_| "mainnet".to_string());
    let region = std::env::var("REGION").unwrap_or_else(|_| "NY".to_string());
    println!("network {}", network);
    println!("region {}", region);
    match (network.as_str(), region.as_str()) {
        ("LOCAL", _) => (LOCAL.to_string(), false),
        ("TESTNET", _) => (TESTNET.to_string(), true),
        ("MAINNET", "UK") => (MAINNET_UK.to_string(), true),
        ("MAINNET", "NY") => (MAINNET_NY.to_string(), true),
        ("MAINNET_PUMP", "NY") => (MAINNET_PUMP_NY.to_string(), true),
        ("MAINNET_PUMP", "UK") => (MAINNET_PUMP_UK.to_string(), true),
        _ => (MAINNET_NY.to_string(), false),
    }
}

pub struct BaseConfig {
    pub keypair: Option<Keypair>,
    pub auth_header: String,
    pub public_key: Option<Pubkey>,
}

impl BaseConfig {
    pub fn try_from_env() -> Result<Self> {
        dotenv().ok();

        let auth_header = env::var("AUTH_HEADER")
            .map_err(|_| anyhow!("AUTH_HEADER environment variable not set"))?;

        let public_key = env::var("PUBLIC_KEY").ok().and_then(|pk_str| {
            Pubkey::from_str(&pk_str)
                .map_err(|e| {
                    println!("Warning: Failed to parse public key: {}", e);
                    e
                })
                .ok()
        });

        let keypair = if let Ok(private_key) = env::var("PRIVATE_KEY") {
            let mut output = [0; 64];
            match decode(private_key).onto(&mut output) {
                Ok(_) => match Keypair::from_bytes(&output) {
                    Ok(kp) => Some(kp),
                    Err(e) => {
                        println!("Warning: Failed to create keypair: {}", e);
                        None
                    }
                },
                Err(e) => {
                    println!("Warning: Failed to decode private key: {}", e);
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            keypair,
            auth_header,
            public_key,
        })
    }
}
