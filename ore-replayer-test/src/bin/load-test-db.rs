mod argos_svm;

use std::{collections::HashSet, hash::Hash, str::FromStr};

use rusqlite::{Connection, OpenFlags, Result};
use solana_client::{nonblocking::rpc_client, pubsub_client::PubsubClient};
use solana_program::{
    instruction::{AccountMeta, CompiledInstruction, Instruction},
    pubkey::Pubkey,
};
use solana_svm::transaction_processor;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ore = Pubkey::from_str("oreV2ZymfyeXgNgBdqMkumTqqAprVqgBWQfoYkrtKWQ")?;
    let client = rpc_client::RpcClient::new(
        "https://mainnet.helius-rpc.com/?api-key=00aaba96-cf5f-40ed-9555-7cafc5a3d85c".to_string(),
    );

    let conn = Connection::open("ore.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS  accounts (
            id         INTEGER PRIMARY KEY,
            slot       INTEGER NOT NULL,
            pubk       TEXT NOT NULL,
            lamport    INTEGER NOT NULL,
            data       BLOB NOT NULL,
            rent_epoch INTEGER NOT NULL,
            owner      TEXT NOT NULL
        )",
        (),
    )?;

    /*
    // this is an approximation hoping it will be ok
    let slot = client.get_slot().await?;
    let accounts = client.get_program_accounts(&ore).await?;
    println!("[+] Got {} accounts for slot {}", accounts.len(), slot);
    for (id, account) in accounts {
        conn.execute(
            "INSERT INTO accounts (slot, pubk, lamport, data, rent_epoch, owner) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                slot as i32,
                id.to_string(),
                account.lamports as i32,
                account.data,
                account.rent_epoch as i32,
                account.owner.to_string(),
            ),
        )?;
    }
    */

    let start_slot = client.get_slot().await?;
    let (mut pub_sub, rx) = PubsubClient::program_subscribe(
        "wss://mainnet.helius-rpc.com/?api-key=00aaba96-cf5f-40ed-9555-7cafc5a3d85c",
        &ore,
        None,
    )?;
    while let Ok(resp) = rx.recv() {
        let slot = resp.context.slot;
        let id = resp.value.pubkey;
        let account = resp.value.account;
        conn.execute(
            "INSERT INTO accounts (slot, pubk, lamport, data, rent_epoch, owner) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                slot as i32,
                id.to_string(),
                account.lamports as i32,
                account.data.decode().unwrap_or_default(),
                account.rent_epoch as i32,
                account.owner.to_string(),
            ),
        )?;
        // stop when we have more than 150 slots (~ 1 minutes)
        if slot - start_slot > 150 {
            break;
        }
    }
    pub_sub.send_unsubscribe()?;
    pub_sub.shutdown().unwrap();

    let mut stmt = conn.prepare("SELECT DITINCT slot from accounts")?;
    let slots: Vec<i32> = stmt
        .query_map([], |row| row.get(0))?
        .filter_map(|s| s.ok())
        .collect();

    // skip 2 first we assume we are getting
    let start_state = {
        let mut stmt = conn.prepare("SELECT id, data from accounts where slot=$?")?;
        let res: Vec<(String, Vec<u8>)> = stmt
            .query_map([slots[1]], |row| {
                let pubkey: String = row.get(0)?;
                let data: Vec<u8> = row.get(1)?;
                Ok((pubkey, data))
            })?
            .filter_map(|r| r.ok())
            .collect();
        res
    };

    for slot in &slots[2..] {
        let end_state = {
            let mut stmt = conn.prepare("SELECT id, data from accounts where slot=$?")?;
            let res: Vec<(String, Vec<u8>)> = stmt
                .query_map([slots.last()], |row| {
                    let pubkey: String = row.get(0)?;
                    let data: Vec<u8> = row.get(1)?;
                    Ok((pubkey, data))
                })?
                .filter_map(|r| r.ok())
                .collect();
            res
        };

        let block = client.get_block(*slot as u64).await?;

        let ore_tx = block.transactions.into_iter().map(|tx| {
            let transaction = tx.transaction.decode().unwrap();
            let accounts = transaction.message.static_account_keys().to_owned();
            let ore_ix: Vec<CompiledInstruction> = transaction
                .message
                .instructions()
                .into_iter()
                .filter(|ix| accounts[ix.program_id_index as usize] == ore)
                .map(|ix| ix.clone())
                .collect();
            (accounts, ore_ix)
        });
    }
    Ok(())
}
