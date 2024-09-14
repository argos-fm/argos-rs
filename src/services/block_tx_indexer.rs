use futures::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_client::SerializableTransaction,
    rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use solana_transaction_status::TransactionDetails;
use sqlx::{Pool, QueryBuilder, Sqlite};

pub async fn block_tx_indexer(db:Pool<Sqlite>, program_id:&Pubkey) {
    let client = PubsubClient::new(crate::SOLANA_BLOCKS_RPC_WS).await.unwrap();
    let block_filter = RpcBlockSubscribeFilter::MentionsAccountOrProgram(program_id.to_string());
    let comitment = CommitmentConfig::confirmed();
    let block_config = RpcBlockSubscribeConfig {
        commitment: Some(comitment),
        encoding: Some(solana_transaction_status::UiTransactionEncoding::Base64),
        transaction_details: Some(TransactionDetails::Full),
        show_rewards: Some(false),
        max_supported_transaction_version: Some(0),
    };

    let (mut block_stream, _resp) = match client
        .block_subscribe(block_filter, Some(block_config))
        .await
    {
        Ok(res) => res,
        Err(err) => {
            tracing::error!("[!] Failed to subscribe to program : {}", err);
            return;
        }
    };

    while let Some(block) = block_stream.next().await {
        let slot = block.value.slot;
        let block = match block.value.block {
            Some(b) => b,
            None => {
                tracing::error!("[!] Missing block");
                continue;
            }
        };
        let block_time = block.block_time;
        let transactions = match block.transactions {
            Some(txs) => txs,
            None => {
                tracing::error!("[!] Missing transactions");
                continue;
            }
        };
        let program_txs = transactions
            .into_iter()
            .filter_map(|tx| tx.transaction.decode().map(|t| (tx.meta, t)));

        let txs_count = program_txs.clone().count();
        tracing::info!("Got block {} |  {:2} txs", slot, txs_count);

        let mut query_builder: QueryBuilder<Sqlite> =
            QueryBuilder::new("INSERT OR REPLACE INTO transactions (signature, slot,  err, memo, block_time, confirmation_status, data) ");
        query_builder.push_values(program_txs, |mut b, (meta, tx)| {
            let signature = tx.get_signature().to_string();
            let data = bincode::serialize(&tx).ok();
            let err = meta.map(|m| m.err.map(|e| e.to_string()));
            // todo: extract memo ?
            let memo: Option<String> = None;
            let block_time = block_time;
            let confirmation_status = Some(comitment.commitment.to_string());
            b.push_bind(signature)
                .push_bind(slot as i32)
                .push_bind(err)
                .push_bind(memo)
                .push_bind(block_time)
                .push_bind(confirmation_status)
                .push_bind(data);
        });
        query_builder.build().execute(&db).await.unwrap();
    }
}
