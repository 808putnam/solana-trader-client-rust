pub mod quote;
pub mod swap;

use anyhow::{anyhow, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::de::DeserializeOwned;
use serde_json::json;
use solana_sdk::{pubkey::Pubkey, signature::Keypair};
use solana_trader_proto::api::{GetRecentBlockHashResponseV2, TransactionMessage};

use crate::{
    common::{
        get_base_url_from_env, http_endpoint,
        signing::{get_keypair, sign_transaction},
        BaseConfig,
    },
    provider::utils::convert_string_enums,
};

pub struct HTTPClient {
    client: Client,
    base_url: String,
    keypair: Option<Keypair>,
    pub public_key: Option<Pubkey>,
}

impl HTTPClient {
    pub fn new(endpoint: Option<String>) -> Result<Self> {
        let base = BaseConfig::try_from_env()?;
        let (base_url, secure) = get_base_url_from_env();
        let endpoint = endpoint.unwrap_or_else(|| http_endpoint(&base_url, secure));

        let headers = Self::build_headers(&base.auth_header)?;
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .map_err(|e| anyhow!("Failed to create HTTP client: {}", e))?;

        Ok(Self {
            client,
            base_url: endpoint,
            keypair: base.keypair,
            public_key: base.public_key,
        })
    }

    fn build_headers(auth_header: &str) -> Result<HeaderMap> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(auth_header)
                .map_err(|e| anyhow!("Invalid auth header: {}", e))?,
        );
        headers.insert("x-sdk", HeaderValue::from_static("rust-client"));
        headers.insert(
            "x-sdk-version",
            HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        );
        Ok(headers)
    }

    async fn handle_response<T: DeserializeOwned>(&self, response: reqwest::Response) -> Result<T> {
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".into());
            return Err(anyhow::anyhow!("HTTP request failed: {}", error_text));
        }

        let res = response.text().await?;

        let mut value = serde_json::from_str(&res)
            .map_err(|e| anyhow::anyhow!("Failed to parse response as JSON: {}", e))?;

        convert_string_enums(&mut value);

        serde_json::from_value(value)
            .map_err(|e| anyhow::anyhow!("Failed to parse response into desired type: {}", e))
    }

    pub async fn sign_and_submit(
        &self,
        tx: TransactionMessage,
        _skip_pre_flight: bool,
        front_running_protection: bool,
        use_staked_rpcs: bool,
        _fast_best_effort: bool,
    ) -> Result<String> {
        let keypair = get_keypair(&self.keypair)?;

        let response = self
            .client
            .get(format!(
                "{}/api/v2/system/blockhash?offset={}",
                self.base_url, 0
            ))
            .send()
            .await?;

        let res: GetRecentBlockHashResponseV2 = self.handle_response(response).await?;
        let signed_tx = sign_transaction(&tx, keypair, res.block_hash).await?;

        let request_json = json!({
            "transaction": { "content": signed_tx.content},
            "frontRunningProtection": front_running_protection,
            "useStakedRPCs": use_staked_rpcs
        });

        let response = self
            .client
            .post(format!("{}/api/v2/submit", self.base_url))
            .json(&request_json)
            .send()
            .await?;

        let result: serde_json::Value = self.handle_response(response).await?;
        result
            .get("signature")
            .and_then(|s| s.as_str())
            .map(String::from)
            .ok_or_else(|| anyhow!("Missing signature in response"))
    }
}
