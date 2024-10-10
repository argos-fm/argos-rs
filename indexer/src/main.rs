mod services;
mod solana_rpc_proxy;

use std::time::Duration;

use axum::{routing::post, Extension, Router};
use services::LimitedRequestClient;
use solana_program::pubkey::Pubkey;
use solana_sdk::pubkey;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const SOLANA_RPC: &str =
    "https://mainnet.helius-rpc.com/?api-key=7ea98130-baee-4b31-94e3-20d9da28ddc3";
pub const SOLANA_ACCOUNT_RPC_WS: &str =
    "wss://mainnet.helius-rpc.com/?api-key=7ea98130-baee-4b31-94e3-20d9da28ddc3";
pub const SOLANA_BLOCKS_RPC_WS : &str = "wss://divine-frequent-log.solana-mainnet.quiknode.pro/9abcf81e71af059e052f4c8f9636cc7536ade363";

//pub const PROGRAM_ID : Pubkey = pubkey!("KswapMzo937QtKugWqNPYcqqiN17XWnbjEqjsEfPZM8");
pub const PROGRAM_ID: Pubkey = pubkey!("oreV2ZymfyeXgNgBdqMkumTqqAprVqgBWQfoYkrtKWQ");

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "argolens_rs=debug,axum=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_url = format!("{}.db?mode=rwc", PROGRAM_ID);
    let db = sqlx::SqlitePool::connect(&db_url)
        .await
        .expect("Can not create db");
    //sqlx::migrate!("./migrations").run(&db).await.unwrap();

    tokio::task::spawn(services::account_indexer(db.clone(), &PROGRAM_ID));
    tokio::task::spawn(services::block_tx_indexer(db.clone(), &PROGRAM_ID));

    let rpc_client = LimitedRequestClient::new(45, Duration::from_secs(1));

    let app = Router::new()
        .route("/", post(solana_rpc_proxy::rpx_proxy))
        .layer(Extension(rpc_client))
        .with_state(db);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}
