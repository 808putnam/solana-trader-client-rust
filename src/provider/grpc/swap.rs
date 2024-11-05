use anyhow::Result;
use solana_trader_proto::api;
use tonic::Request;

use super::GrpcClient;

impl GrpcClient {
    pub async fn post_raydium_swap(
        &mut self,
        request: &api::PostRaydiumSwapRequest,
    ) -> Result<api::PostRaydiumSwapResponse> {
        let response = self
            .client
            .post_raydium_swap(Request::new(request.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("PostRaydiumSwap error: {}", e))?;

        Ok(response.into_inner())
    }

    pub async fn post_raydium_cpmm_swap(
        &mut self,
        request: &api::PostRaydiumCpmmSwapRequest,
    ) -> Result<api::PostRaydiumCpmmSwapResponse> {
        let response = self
            .client
            .post_raydium_cpmm_swap(Request::new(request.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("PostRaydiumCPMMSwap error: {}", e))?;

        Ok(response.into_inner())
    }

    pub async fn post_raydium_clmm_swap(
        &mut self,
        request: &api::PostRaydiumSwapRequest,
    ) -> Result<api::PostRaydiumSwapResponse> {
        let response = self
            .client
            .post_raydium_clmm_swap(Request::new(request.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("PostRaydiumCLMMSwap error: {}", e))?;

        Ok(response.into_inner())
    }

    pub async fn post_pump_swap(
        &mut self,
        request: &api::PostPumpFunSwapRequest,
    ) -> Result<api::PostPumpFunSwapResponse> {
        let response = self
            .client
            .post_pump_fun_swap(Request::new(request.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("PostPumpFunSwap error: {}", e))?;

        Ok(response.into_inner())
    }

    pub async fn post_jupiter_swap(
        &mut self,
        request: &api::PostJupiterSwapRequest,
    ) -> Result<api::PostJupiterSwapResponse> {
        let response = self
            .client
            .post_jupiter_swap(Request::new(request.clone()))
            .await
            .map_err(|e| anyhow::anyhow!("PostJupiterSwap error: {}", e))?;

        Ok(response.into_inner())
    }
}
