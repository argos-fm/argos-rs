use axum::{Extension, Json};
use http::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::SqlitePool;

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

#[derive(Debug)]
enum ProxyError {
    Database(sqlx::Error),
    Client(reqwest::Error),
    BadRequest(String),
    InternalServer,
}

impl From<ProxyError> for StatusCode {
    fn from(error: ProxyError) -> Self {
        match error {
            ProxyError::Database(_) | ProxyError::InternalServer => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            ProxyError::Client(_) => StatusCode::BAD_GATEWAY,
            ProxyError::BadRequest(_) => StatusCode::BAD_REQUEST,
        }
    }
}

async fn get_account_from_db(
    pool: &SqlitePool,
    account_id: &str,
) -> Result<Option<Value>, ProxyError> {
    sqlx::query!(
        "SELECT data, slot, executable, lamports, owner, rent_epoch FROM accounts_archive WHERE id = ?",
        account_id
    )
    .fetch_optional(pool)
    .await
    .map(|row| if let Some(row) = row {
        Some(
            json!({
                "data":row.data,
                "slot":row.slot,
                "executable":row.executable,
                "lamports":row.lamports,
                "owner":row.owner,
                "rentEpoch":row.rent_epoch
            })
        )
    } else {
            None
        }
    )
    .map_err(|err| ProxyError::Database(err))
}

#[axum::debug_handler]
pub async fn rpx_proxy(
    Extension(client): Extension<LimitedRequestClient>,
    Extension(pool): Extension<SqlitePool>,
    Json(request): Json<RpcRequest>,
) -> Result<Json<Value>, StatusCode> {
    tracing::debug!("Got request {:?}", request);

    let resp = match request.method {
        RpcMethod::GetVersion => {
            json!({"jsonrpc":"2.0","result":{"feature-set":2891131721u64,"solana-core":"1.16.7"},"id":request.id})
        }
        RpcMethod::GetAccountInfo(params) => {
            let account_id = params[0]
                .as_str()
                .ok_or(ProxyError::BadRequest("Invalid account ID".into()))?;
            let config = params.get(1).and_then(|v| v.as_object());

            if let Some(account) = get_account_from_db(&pool, account_id).await? {
                json!({
                    "cached":account,
                })
            } else {
                client
                    .proxy_request(json!({
                        "jsonrpc": "2.0",
                        "id": request.id,
                        "method": "getAccountInfo",
                        "params": params
                    }))
                    .await
                    .map_err(|err| ProxyError::BadRequest(err))?
            }
        }
        RpcMethod::GetProgramAccounts(_) => todo!(),
        RpcMethod::GetSignaturesForAddress(_) => todo!(),
        RpcMethod::Unproxied(v) => {
            tracing::info!("Unproxied request {:?}", v);
            client
                .proxy_request(v)
                .await
                .map_err(|err| ProxyError::BadRequest(err))?
        }
    };

    Ok(Json(resp))
}
