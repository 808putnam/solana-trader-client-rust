use crate::{
    common::signing::get_keypair,
    provider::utils::{
        convert_address_lookup_table, convert_jupiter_instructions, convert_raydium_instructions,
        create_transaction_message,
    },
};

use super::HTTPClient;
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use solana_sdk::{
    message::{v0, VersionedMessage},
    transaction::VersionedTransaction,
};
use solana_trader_proto::api;

impl HTTPClient {
    pub async fn post_raydium_swap(
        &self,
        request: &api::PostRaydiumSwapRequest,
    ) -> Result<api::PostRaydiumSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/raydium/swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_raydium_route_swap(
        &self,
        request: &api::PostRaydiumRouteSwapRequest,
    ) -> Result<api::PostRaydiumRouteSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/raydium/route-swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_raydium_swap_instructions(
        &self,
        request: &api::PostRaydiumSwapInstructionsRequest,
    ) -> Result<api::PostRaydiumSwapInstructionsResponse> {
        let response = self
            .client
            .post(format!(
                "{}/api/v2/raydium/swap-instructions",
                self.base_url
            ))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn submit_raydium_swap_instructions(
        &self,
        request: api::PostRaydiumSwapInstructionsRequest,
        use_bundle: bool,
    ) -> Result<Vec<String>> {
        // Get the swap instructions
        let swap_instructions = self.post_raydium_swap_instructions(&request).await?;

        // Convert instructions to Solana format using shared utilities
        let instructions = convert_raydium_instructions(&swap_instructions.instructions)?;

        // Get recent blockhash
        let response = self
            .client
            .get(format!(
                "{}/api/v2/system/blockhash?offset=0",
                self.base_url
            ))
            .send()
            .await?;

        let blockhash_response: api::GetRecentBlockHashResponseV2 =
            self.handle_response(response).await?;

        // Create transaction message
        let tx_message = create_transaction_message(instructions, &blockhash_response.block_hash)?;

        // Sign and submit using existing HTTPClient implementation
        self.sign_and_submit(
            vec![tx_message],
            false, // skip_pre_flight
            false,  // front_running_protection
            true,  // use_staked_rpcs
            false, // fast_best_effort
            use_bundle,
        )
        .await
    }

    pub async fn post_raydium_cpmm_swap(
        &self,
        request: &api::PostRaydiumCpmmSwapRequest,
    ) -> Result<api::PostRaydiumCpmmSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/raydium/cpmm-swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_raydium_clmm_swap(
        &self,
        request: &api::PostRaydiumSwapRequest,
    ) -> Result<api::PostRaydiumSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/raydium/clmm-swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_raydium_clmm_route_swap(
        &self,
        request: &api::PostRaydiumRouteSwapRequest,
    ) -> Result<api::PostRaydiumRouteSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/raydium/clmm-route-swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_jupiter_swap(
        &self,
        request: &api::PostJupiterSwapRequest,
    ) -> Result<api::PostJupiterSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/jupiter/swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_jupiter_route_swap(
        &self,
        request: &api::PostJupiterRouteSwapRequest,
    ) -> Result<api::PostJupiterRouteSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/jupiter/route-swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_jupiter_swap_instructions(
        &self,
        request: &api::PostJupiterSwapInstructionsRequest,
    ) -> Result<api::PostJupiterSwapInstructionsResponse> {
        let response = self
            .client
            .post(format!(
                "{}/api/v2/jupiter/swap-instructions",
                self.base_url
            ))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn submit_jupiter_swap_instructions(
        &self,
        request: api::PostJupiterSwapInstructionsRequest,
        use_bundle: bool,
    ) -> Result<Vec<String>> {
        let keypair = get_keypair(&self.keypair)?;

        // Get the swap instructions
        let swap_instructions = self.post_jupiter_swap_instructions(&request).await?;

        // Convert address lookup tables
        let address_lookup_tables =
            convert_address_lookup_table(&swap_instructions.address_lookup_table_addresses)?;

        // Convert instructions to Solana format
        let instructions = convert_jupiter_instructions(&swap_instructions.instructions)?;

        // Get recent blockhash
        let response = self
            .client
            .get(format!(
                "{}/api/v2/system/blockhash?offset=0",
                self.base_url
            ))
            .send()
            .await?;

        let blockhash_response: api::GetRecentBlockHashResponseV2 =
            self.handle_response(response).await?;

        let message = VersionedMessage::V0(v0::Message::try_compile(
            &self.public_key.unwrap(),
            &instructions,
            &address_lookup_tables,
            blockhash_response.block_hash.parse()?,
        )?);

        let tx: VersionedTransaction = VersionedTransaction::try_new(message, &[keypair])?;

        // Convert to transaction message
        let tx_message = api::TransactionMessage {
            content: general_purpose::STANDARD.encode(bincode::serialize(&tx)?),
            is_cleanup: false,
        };

        // Sign and submit
        self.sign_and_submit(
            vec![tx_message],
            false, // skip_pre_flight
            false, // front_running_protection
            true,  // use_staked_rpcs
            false, // fast_best_effort
            use_bundle,
        )
        .await
    }

    pub async fn post_trade_swap(
        &self,
        request: &api::TradeSwapRequest,
    ) -> Result<api::TradeSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/trade/swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }

    pub async fn post_route_trade_swap(
        &self,
        request: &api::RouteTradeSwapRequest,
    ) -> Result<api::TradeSwapResponse> {
        let response = self
            .client
            .post(format!("{}/api/v2/trade/route-swap", self.base_url))
            .json(&request)
            .send()
            .await?;

        self.handle_response(response).await
    }
}
