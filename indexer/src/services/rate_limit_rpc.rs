use http::{header::CONTENT_TYPE, HeaderValue, Method};
use rand::{thread_rng, Rng};
use reqwest::{Client, Request, Response, Url};
use serde_json::{json, Value};
use solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};
use tower::{Service, ServiceExt};

use crate::SOLANA_RPC;

// A simple type alias so as to DRY.
type Result<T> = std::result::Result<T, String>;

#[derive(Debug, Clone)]
pub struct LimitedRequestClient {
    request_tx: mpsc::UnboundedSender<(Request, oneshot::Sender<Result<Response>>)>,
}

impl LimitedRequestClient {
    pub fn new(rate_limit_number: u64, rate_limit_duration: Duration) -> Self {
        let reqwest_client = reqwest::Client::builder().build().unwrap();
        let (tx, mut rx) =
            mpsc::unbounded_channel::<(Request, oneshot::Sender<Result<Response>>)>();

        tokio::spawn(async move {
            let mut service = tower::ServiceBuilder::new()
                .rate_limit(rate_limit_number, rate_limit_duration)
                .service(reqwest_client);
            while let Some((req, resp_tx)) = rx.recv().await {
                let srv = service.ready().await.unwrap();
                let resp = srv.call(req);
                tokio::spawn(async {
                    let resp = resp.await.map_err(|err| err.to_string());
                    resp_tx.send(resp)
                });
            }
        });
        Self { request_tx: tx }
    }

    pub async fn get_blocks(&self, start_slot: u64, end_slot: Option<u64>) -> Result<Response> {
        let rand_id: usize = thread_rng().gen();
        let body_value = json!({
            "jsonrpc": "2.0",
            "id":rand_id,
            "method":"getBlocks",
            "params": [start_slot, end_slot],
        });

        let mut request = Request::new(Method::POST, Url::parse(SOLANA_RPC).unwrap());
        request
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let body = serde_json::to_vec(&body_value).unwrap();
        *request.body_mut() = Some(body.into());
        self.request(request).await
    }

    pub async fn get_block(&self, slot: u64) -> Result<Response> {
        let rand_id: usize = thread_rng().gen();
        let body_value = json!({
            "jsonrpc": "2.0",
            "id":rand_id,
            "method":"getBlock",
            "params": [
                slot,
                {
                "encoding": "base64",
                "maxSupportedTransactionVersion":0,
                "transactionDetails":"full",
                "rewards":false
                }
            ]
        });

        let mut request = Request::new(Method::POST, Url::parse(SOLANA_RPC).unwrap());
        request
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let body = serde_json::to_vec(&body_value).unwrap();
        *request.body_mut() = Some(body.into());
        self.request(request).await
    }

    pub async fn get_block_accounts(&self, slot: u64) -> Result<Response> {
        let rand_id: usize = thread_rng().gen();
        let body_value = json!({
            "jsonrpc": "2.0",
            "id":rand_id,
            "method":"getBlock",
            "params": [
                slot,
                {
                "encoding": "base64",
                "maxSupportedTransactionVersion":0,
                "transactionDetails":"accounts",
                "rewards":false
                }
            ]
        });

        let mut request = Request::new(Method::POST, Url::parse(SOLANA_RPC).unwrap());
        request
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let body = serde_json::to_vec(&body_value).unwrap();
        *request.body_mut() = Some(body.into());
        self.request(request).await
    }

    pub async fn get_transaction(
        &self,
        signature: String,
    ) -> Result<EncodedConfirmedTransactionWithStatusMeta> {
        let rand_id: usize = thread_rng().gen();
        let body_value = json!({
            "jsonrpc": "2.0",
            "id":rand_id,
            "method":"getTransaction",
            "params": [
                signature,
                {
                    "encoding":"base64",
                    "maxSupportedTransactionVersion":0,
                },
            ]
        });

        let mut request = Request::new(Method::POST, Url::parse(SOLANA_RPC).unwrap());
        request
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let body = serde_json::to_vec(&body_value).map_err(|err| err.to_string())?;
        *request.body_mut() = Some(body.into());

        let resp = self.request(request).await.map_err(|err| err.to_string())?;
        let rpc_resp: Value = resp.json().await.map_err(|err| err.to_string())?;

        if let Some(err) = rpc_resp.get("error") {
            return Err(format!("[!] Block error {:?}", err));
        }

        let tx_json = rpc_resp
            .get("result")
            .ok_or("result not found".to_string())?;
        let tx: EncodedConfirmedTransactionWithStatusMeta =
            serde_json::from_value(tx_json.to_owned()).map_err(|err| err.to_string())?;
        Ok(tx)
    }

    pub async fn proxy_request(&self, body_value: Value) -> Result<Value> {
        let mut request = Request::new(Method::POST, Url::parse(SOLANA_RPC).unwrap());
        request
            .headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let body = serde_json::to_vec(&body_value).map_err(|err| err.to_string())?;
        *request.body_mut() = Some(body.into());

        let resp = self.request(request).await.map_err(|err| err.to_string())?;
        let rpc_resp: Value = resp.json().await.map_err(|err| err.to_string())?;
        Ok(rpc_resp)
    }

    async fn request(&self, req: Request) -> Result<Response> {
        let (tx, rx) = oneshot::channel::<Result<Response>>();
        self.request_tx
            .send((req, tx))
            .map_err(|err| err.to_string())?;
        rx.await.map_err(|err| err.to_string())?
    }
}
