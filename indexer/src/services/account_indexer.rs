use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
};
use solana_sdk::pubkey::Pubkey;
use sqlx::{QueryBuilder, Sqlite, SqlitePool};

pub async fn account_indexer(db: SqlitePool, program_id: &Pubkey) {
    let rpc_client = RpcClient::new(crate::SOLANA_RPC.to_string());
    let slot = rpc_client.get_slot().await.expect("faild to get slot");
    let config = RpcProgramAccountsConfig {
        sort_results: None,
        filters: None,
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64Zstd),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: Some(true),
    };
    tracing::info!("Fetching accounts");
    let accounts = rpc_client
        .get_program_accounts_with_config(program_id, config)
        .await
        .expect("Failed to fetch accounts");
    tracing::info!("Got {} accounts", accounts.len());
    for chunk in accounts.chunks(10_000) {
        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT or REPLACE INTO accounts_archive(id, slot, data) ");
        query_builder.push_values(chunk, |mut b, (id, account)| {
            b.push_bind(id.to_string())
                .push_bind(slot as i32)
                .push_bind(account.data.clone());
        });

        query_builder
            .build()
            .execute(&db)
            .await
            .expect("Accouts bulk insert failed");
        tracing::info!("Indexed {} accounts", chunk.len());
    }

    let sub_config = RpcProgramAccountsConfig {
        sort_results: None,
        filters: None,
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            data_slice: None,
            commitment: None,
            min_context_slot: None,
        },
        with_context: Some(true),
    };
    let (_tx, rx) = solana_client::pubsub_client::PubsubClient::program_subscribe(
        crate::SOLANA_ACCOUNT_RPC_WS,
        program_id,
        Some(sub_config),
    )
    .unwrap();

    while let Ok(msg) = rx.recv() {
        let slot = msg.context.slot as i64;
        if let Some(data) = msg.value.account.data.decode() {
            sqlx::query!(
                "INSERT or REPLACE into accounts_archive (id, slot, data) VALUES ($1, $2, $3)",
                msg.value.pubkey,
                slot,
                data,
            )
            .execute(&db)
            .await
            .expect("Failt to insert account");
            tracing::info!("Updated account {}", msg.value.pubkey);
        } else {
            tracing::error!("Failed to decode account {}", msg.value.pubkey);
        }
    }
}
