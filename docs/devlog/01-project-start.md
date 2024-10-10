# Ore indexer

## context
Solana ledger contains transaction history.
A transactions is composed of multiple instructions.
An instruction call a program with a list of accounts and arbytraty data.
Solana program store and update on chain data inside accounts.

Our goal will be to access history of accounts change between transactions
This data is unvailable.

## The plan

As a cool and usefull example we are going to index the Ore program.
Ore is a mining program
The Ore program store insisde acounts how many token a miner as mined.
A user can unstack those token to its wallet or keep them stack a gain a bonus.
Our goal will be to access how many Ore token every minner has minned and when.

You can get dificulty minned and total amount unstacked.
But not whene was the ore Minned.

Here are the step we are going to follow

- Get state change and transaction for 1 hour
- Display simple graph per minner
- Replay tx using svm and check for state diff correctness
- Handle program upgrade
- DL all tx
- Replay everything

## step one

Get ore pubkey from github : oreV2ZymfyeXgNgBdqMkumTqqAprVqgBWQfoYkrtKWQ
Use helius RPC to get current accoutns.
We will store all accounts in sqlite database.

```
cargo new ore-indexer
# add solana client
cargo add solana_client
# add sqlite 
cargo add rusqlite -F "bundled" 
```

