use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::de::DeserializeOwned;
use solana_sdk::signature::Keypair;
use solana_trader_proto::api;
use std::time::Duration;

use crate::provider::{
    constants::LOCAL_HTTP,
    error::{ClientError, Result},
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug)]
pub struct HTTPClient {
    client: Client,
    base_url: String,
    private_key: Option<Keypair>,
    auth_header: String,
}

#[derive(Debug)]
pub struct HTTPClientConfig {
    pub endpoint: String,
    pub private_key: Option<Keypair>,
    pub auth_header: String,
    pub timeout: Option<Duration>,
}

impl HTTPClientConfig {
    pub fn try_from_env() -> Result<Self> {
        let private_key = std::env::var("PRIVATE_KEY")
            .ok()
            .map(|pk| Keypair::from_base58_string(&pk));

        let auth_header = std::env::var("AUTH_HEADER").map_err(|_| {
            ClientError::from(String::from("AUTH_HEADER environment variable not set"))
        })?;

        Ok(Self {
            endpoint: LOCAL_HTTP.to_string(),
            private_key,
            auth_header,
            timeout: Some(DEFAULT_TIMEOUT),
        })
    }
}

impl HTTPClient {
    pub fn new(endpoint: String) -> Result<Self> {
        let mut config = HTTPClientConfig::try_from_env()?;
        config.endpoint = endpoint;
        Self::with_config(config)
    }

    pub fn with_config(config: HTTPClientConfig) -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "Authorization",
            HeaderValue::from_str(&config.auth_header)
                .map_err(|e| ClientError::new("Invalid auth header", e))?,
        );
        headers.insert("x-sdk", HeaderValue::from_static("rust-client"));
        headers.insert(
            "x-sdk-version",
            HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        );

        let client = Client::builder()
            .default_headers(headers)
            .timeout(config.timeout.unwrap_or(DEFAULT_TIMEOUT))
            .build()
            .map_err(|e| ClientError::new("Failed to create HTTP client", e))?;

        Ok(Self {
            client,
            base_url: config.endpoint,
            private_key: config.private_key,
            auth_header: config.auth_header,
        })
    }

    async fn handle_response<T>(&self, response: reqwest::Response) -> Result<T>
    where
        T: DeserializeOwned,
    {
        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| String::from("Failed to read error response"));
            return Err(ClientError::new("HTTP request failed", error_text));
        }

        response
            .json::<T>()
            .await
            .map_err(|e| ClientError::new("Failed to parse response", e))
    }

    pub async fn get_raydium_quotes(
        &self,
        request: &api::GetRaydiumQuotesRequest,
    ) -> Result<api::GetRaydiumQuotesResponse> {
        let url = format!(
            "{}/quotes?inToken={}&outToken={}&inAmount={}&slippage={}",
            self.base_url, request.in_token, request.out_token, request.in_amount, request.slippage
        );

        println!("Making request to: {}", url); // Debug line

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| ClientError::new("HTTP GET request failed", e))?;

        self.handle_response(response).await
    }
}
