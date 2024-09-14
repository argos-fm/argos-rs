use axum::{Extension, Json};
use http::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::services::LimitedRequestClient;

#[derive(Deserialize, Debug)]
pub struct RpcRequest {
    id: u64,
    jsonrpc: String,
    #[serde(flatten)]
    method: RpcMethod,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", tag = "method", content = "params")]
enum RpcMethod {
    GetVersion,
    GetAccountInfo(Value),
    GetProgramAccounts(Value),
    GetSignaturesForAddress(Value),
    #[serde(untagged)]
    Unproxied(Value),
}

#[axum::debug_handler]
pub async fn rpx_proxy(Extension(client): Extension<LimitedRequestClient>, Json(request):Json<RpcRequest>) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Got request {:?}", request);

    let resp = match request.method {
        RpcMethod::GetVersion => {
            json!({"jsonrpc":"2.0","result":{"feature-set":2891131721u64,"solana-core":"1.16.7"},"id":request.id})
        },
        RpcMethod::GetAccountInfo(_) => todo!(),
        RpcMethod::GetProgramAccounts(_) => todo!(),
        RpcMethod::GetSignaturesForAddress(_) => todo!(),
        RpcMethod::Unproxied(v) => {
            tracing::info!("Unproxied request {:?} ", v);
            client.proxy_request(v).await.map_err(|err|{
                tracing::error!("proxy request failed ({})", err);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        }
    };

    Ok(Json(resp))
}
