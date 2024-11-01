use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use rustls::crypto::ring::default_provider;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::{any::Any, time::Duration};
use tokio::net::TcpStream;
use tokio::sync::mpsc::Sender;
use tokio::sync::{broadcast, mpsc, Mutex};
use tokio::time::timeout;
use tokio_rustls::rustls::{ClientConfig, RootCertStore};
use tokio_stream::Stream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::{connect_async_tls_with_config, Connector};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use url::Url;

use crate::common::{get_base_url_from_env, ws_endpoint, BaseConfig};
use crate::provider::utils::convert_string_enums;

const CONNECTION_RETRY_TIMEOUT: Duration = Duration::from_secs(15);
const CONNECTION_RETRY_INTERVAL: Duration = Duration::from_millis(100);
const SUBSCRIPTION_BUFFER: usize = 1000;
const PING_INTERVAL: Duration = Duration::from_secs(30);

pub struct SubscriptionEntry<T> {
    pub active: bool,
    pub sender: mpsc::Sender<T>,
}

pub trait AnySubscription: Send + Sync {
    fn is_active(&self) -> bool;
    fn set_active(&mut self, active: bool);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Clone + Send + Sync + 'static> AnySubscription for SubscriptionEntry<T> {
    fn is_active(&self) -> bool {
        self.active
    }

    fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Clone)]
struct ResponseUpdate {
    response: String,
}

struct RequestTracker {
    ch: Sender<ResponseUpdate>,
}

#[derive(Serialize)]
struct StreamRequest<T> {
    stream_name: String,
    params: T,
}

pub struct WS {
    stream: Arc<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>>,
    write_tx: Sender<Message>,
    shutdown_tx: broadcast::Sender<()>,
    request_id: AtomicU64,
    request_map: Arc<Mutex<HashMap<u64, RequestTracker>>>,
    subscriptions: Arc<Mutex<HashMap<String, Box<dyn AnySubscription + Send + Sync>>>>,
}

impl WS {
    pub async fn new(endpoint: Option<String>) -> Result<Self> {
        let base = BaseConfig::try_from_env()?;
        let (base_url, secure) = get_base_url_from_env();
        let endpoint = endpoint.unwrap_or_else(|| ws_endpoint(&base_url, secure));

        if base.auth_header.is_empty() {
            return Err(anyhow::anyhow!("AUTH_HEADER is empty"));
        }

        let url =
            Url::parse(&endpoint).map_err(|e| anyhow::anyhow!("Invalid WebSocket URL: {}", e))?;

        let stream = Self::connect(&url, &base.auth_header).await?;
        let stream = Arc::new(Mutex::new(stream));

        let (write_tx, write_rx) = mpsc::channel(100);
        let (shutdown_tx, _) = broadcast::channel(1);

        let ws = Self {
            stream: stream.clone(),
            write_tx,
            shutdown_tx,
            request_id: AtomicU64::new(0),
            request_map: Arc::new(Mutex::new(HashMap::new())),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        };

        ws.start_loops(stream, write_rx);
        Ok(ws)
    }

    async fn connect(
        url: &Url,
        auth_header: &str,
    ) -> Result<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>> {
        let mut request = url
            .as_str()
            .into_client_request()
            .map_err(|e| anyhow::anyhow!("Failed to build request: {}", e))?;

        let headers = request.headers_mut();
        headers.insert("Authorization", auth_header.parse()?);
        headers.insert("x-sdk", "rust-client".parse()?);
        headers.insert("x-sdk-version", env!("CARGO_PKG_VERSION").parse()?);
        headers.insert("Connection", "Upgrade".parse()?);
        headers.insert("Upgrade", "websocket".parse()?);
        headers.insert("Sec-WebSocket-Version", "13".parse()?);

        default_provider()
            .install_default()
            .map_err(|e| anyhow::anyhow!("Failed to install crypto provider: {:?}", e))?;

        let root_store = RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        let tls_config = ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let connector = Arc::new(tls_config);

        let mut retry_count = 0;
        let max_retries =
            (CONNECTION_RETRY_TIMEOUT.as_millis() / CONNECTION_RETRY_INTERVAL.as_millis()) as u32;

        loop {
            match connect_async_tls_with_config(
                request.clone(),
                Some(WebSocketConfig::default()),
                true,
                Some(Connector::Rustls(connector.clone())),
            )
            .await
            {
                Ok((stream, _)) => {
                    println!("Connected to: {}", url);
                    return Ok(stream);
                }
                Err(e) => {
                    if retry_count >= max_retries {
                        return Err(anyhow::anyhow!(
                            "WebSocket connection failed after {} retries: {}",
                            max_retries,
                            e
                        ));
                    }
                    retry_count += 1;
                    tokio::time::sleep(CONNECTION_RETRY_INTERVAL).await;
                }
            }
        }
    }

    fn start_loops(
        &self,
        stream: Arc<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>>,
        write_rx: mpsc::Receiver<Message>,
    ) {
        let write_stream = stream.clone();
        tokio::spawn(write_loop(write_stream, write_rx));

        let read_stream = stream.clone();
        let request_map = self.request_map.clone();
        let subscriptions = self.subscriptions.clone();
        tokio::spawn(read_loop(read_stream, request_map, subscriptions));

        let ping_stream = stream;
        let shutdown_rx = self.shutdown_tx.subscribe();
        tokio::spawn(ping_loop(ping_stream, shutdown_rx));
    }

    pub async fn request<T>(&self, method: &str, params: Value) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let request_id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request_json = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        });

        let (tx, mut rx) = mpsc::channel(1);
        {
            let mut request_map = self.request_map.lock().await;
            request_map.insert(request_id, RequestTracker { ch: tx });
        }

        let msg = Message::Text(request_json.to_string());
        timeout(Duration::from_secs(5), self.write_tx.send(msg))
            .await
            .map_err(|_| anyhow::anyhow!("Request send timeout"))?
            .map_err(|e| anyhow::anyhow!("Failed to send request: {}", e))?;

        let response = timeout(Duration::from_secs(10), rx.recv())
            .await
            .map_err(|_| anyhow::anyhow!("Response timeout"))?
            .ok_or_else(|| anyhow::anyhow!("Channel closed unexpectedly"))?;

        let mut json_response: Value = serde_json::from_str(&response.response)
            .map_err(|e| anyhow::anyhow!("Failed to parse response: {}", e))?;

        if let Some(error) = json_response.get("error") {
            return Err(anyhow::anyhow!("RPC error: {}", error));
        }

        // tradeFeeRate come in as a string, and so causes a parsing error without casting to the expected u64.
        if let Some(obj) = json_response.get_mut("result") {
            if let Some(fee_rate) = obj.get("tradeFeeRate") {
                if let Some(fee_str) = fee_rate.as_str() {
                    if let Ok(fee_num) = fee_str.parse::<u64>() {
                        obj["tradeFeeRate"] = json!(fee_num);
                    }
                }
            }
        }

        let result = json_response
            .get("result")
            .ok_or_else(|| anyhow::anyhow!("Missing result field in response"))?;

        let mut res = result.clone();
        convert_string_enums(&mut res);

        serde_json::from_value(res).map_err(|e| anyhow::anyhow!("Failed to parse result: {}", e))
    }

    pub async fn stream_proto<Req, Resp>(
        &self,
        method: &str,
        request: &Req,
    ) -> Result<impl Stream<Item = Result<Resp>> + Unpin>
    where
        Req: prost::Message + Serialize,
        Resp: prost::Message + Default + DeserializeOwned + Send + Clone + 'static,
    {
        let (tx, rx) = mpsc::channel(SUBSCRIPTION_BUFFER);

        let params = serde_json::to_value(request)
            .map_err(|e| anyhow::anyhow!("Failed to serialize request: {}", e))?;

        let params_array = json!([method, params]);
        let subscription_id: String = self.request("subscribe", params_array).await?;

        {
            let mut subs = self.subscriptions.lock().await;
            subs.insert(
                subscription_id,
                Box::new(SubscriptionEntry {
                    active: true,
                    sender: tx,
                }),
            );
        }

        Ok(
            tokio_stream::wrappers::ReceiverStream::new(rx).map(|value: Value| {
                let mut modified_value = value;
                convert_string_enums(&mut modified_value);

                serde_json::from_value(modified_value)
                    .map_err(|e| anyhow::anyhow!("Failed to parse stream value: {}", e))
            }),
        )
    }

    pub async fn close(self) -> Result<()> {
        let _ = self.shutdown_tx.send(());

        {
            let mut request_map = self.request_map.lock().await;
            for (_, tracker) in request_map.drain() {
                let _ = tracker
                    .ch
                    .send(ResponseUpdate {
                        response: String::from("{\"error\":\"connection closed\"}"),
                    })
                    .await;
            }
        }

        let mut stream = self.stream.lock().await;
        if let Err(e) = stream.close(None).await {
            eprintln!("Error during WebSocket close: {}", e);
        }
        drop(stream);

        tokio::time::sleep(Duration::from_millis(100)).await;
        println!("WebSocket shutdown complete");
        Ok(())
    }
}

async fn write_loop(
    stream: Arc<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>>,
    mut write_rx: mpsc::Receiver<Message>,
) {
    while let Some(msg) = write_rx.recv().await {
        let mut stream = stream.lock().await;
        if let Err(e) = stream.send(msg).await {
            eprintln!("Write error: {}", e);
            break;
        }
    }
}

async fn read_loop(
    stream: Arc<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>>,
    request_map: Arc<Mutex<HashMap<u64, RequestTracker>>>,
    subscriptions: Arc<Mutex<HashMap<String, Box<dyn AnySubscription + Send + Sync>>>>,
) {
    loop {
        let mut stream = stream.lock().await;
        match timeout(Duration::from_millis(100), stream.next()).await {
            Ok(Some(Ok(Message::Text(text)))) => match serde_json::from_str(&text) {
                Ok(value) => {
                    handle_message(&value, &request_map, &subscriptions, &text).await;
                }
                Err(e) => {
                    eprintln!("Failed to parse message as JSON: {}", e);
                }
            },
            Ok(Some(Ok(Message::Close(_)))) => break,
            Ok(Some(Err(e))) => {
                eprintln!("WebSocket error: {}", e);
                break;
            }
            Ok(None) => break,
            _ => continue,
        }
    }
}

async fn ping_loop(
    stream: Arc<Mutex<WebSocketStream<tokio_tungstenite::MaybeTlsStream<TcpStream>>>>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let mut interval = tokio::time::interval(PING_INTERVAL);
    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mut stream = stream.lock().await;
                if let Err(e) = stream.send(Message::Ping(vec![])).await {
                    eprintln!("Ping error: {}", e);
                    break;
                }
            }
            Ok(_) = shutdown_rx.recv() => break,
        }
    }
}

async fn handle_message(
    value: &Value,
    request_map: &Arc<Mutex<HashMap<u64, RequestTracker>>>,
    subscriptions: &Arc<Mutex<HashMap<String, Box<dyn AnySubscription + Send + Sync>>>>,
    text: &str,
) {
    if let Some(id) = value.get("id").and_then(|id| id.as_u64()) {
        let request_map = request_map.lock().await;
        if let Some(tracker) = request_map.get(&id) {
            let _ = tracker
                .ch
                .send(ResponseUpdate {
                    response: text.to_string(),
                })
                .await;
        }
        return;
    }

    handle_subscription(value, subscriptions).await;
}

async fn handle_subscription(
    map: &Value,
    subscriptions: &Arc<Mutex<HashMap<String, Box<dyn AnySubscription + Send + Sync>>>>,
) {
    let subscription_id = match map
        .get("params")
        .and_then(|params| params.get("subscription"))
        .and_then(|sub| sub.as_str())
    {
        Some(id) => id,
        None => return,
    };

    let subs = subscriptions.lock().await;
    let sub = match subs.get(subscription_id) {
        Some(s) if s.is_active() => s,
        _ => return,
    };

    if let Some(params) = map.get("params") {
        if let Some(result) = params.get("result") {
            if let Some(entry) = sub.as_any().downcast_ref::<SubscriptionEntry<Value>>() {
                let _ = entry.sender.send(result.clone()).await;
            }
        }
    }
}
