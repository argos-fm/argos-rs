use solana_client::{nonblocking::rpc_client, rpc_client::RpcClient};
use solana_program::clock::Slot;
use solana_program_runtime::loaded_programs::{
    BlockRelation, ForkGraph, LoadProgramMetrics, ProgramCacheEntry,
};
use solana_sdk::{
    account::{AccountSharedData, ReadableAccount},
    pubkey::Pubkey,
};
use solana_svm::{
    transaction_processing_callback::TransactionProcessingCallback,
    transaction_processor::TransactionBatchProcessor,
};
use solana_system_program::system_processor;
use std::{
    collections::{HashMap, HashSet},
    sync::RwLock,
};

struct ArgosForkGraph {}

impl ForkGraph for ArgosForkGraph {
    fn relationship(&self, _a: Slot, _b: Slot) -> BlockRelation {
        BlockRelation::Unknown
    }
}

struct ArgosAccountLoader<'a> {
    cache: RwLock<HashMap<Pubkey, AccountSharedData>>,
    rpc_client: &'a RpcClient,
}

impl<'a> ArgosAccountLoader<'a> {
    pub fn new(rpc_client: &'a RpcClient) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            rpc_client,
        }
    }
}

impl TransactionProcessingCallback for ArgosAccountLoader<'_> {
    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        if let Some(account) = self.cache.read().unwrap().get(pubkey) {
            return Some(account.clone());
        }

        let account: AccountSharedData = self.rpc_client.get_account(pubkey).ok()?.into();
        self.cache.write().unwrap().insert(*pubkey, account.clone());

        Some(account)
    }

    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        self.get_account_shared_data(account)
            .and_then(|account| owners.iter().position(|key| account.owner().eq(key)))
    }
}

pub fn create_svm() {
    let processor = TransactionBatchProcessor::<ArgosForkGraph>::default();
    let rpc_client = RpcClient::new(
        "https://mainnet.helius-rpc.com/?api-key=00aaba96-cf5f-40ed-9555-7cafc5a3d85c",
    );
    let loader = ArgosAccountLoader::new(&rpc_client);

    // Add the system program builtin.
    processor.add_builtin(
        &loader,
        solana_system_program::id(),
        "system_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"system_program".len(),
            system_processor::Entrypoint::vm,
        ),
    );

    // Add the BPF Loader v2 builtin, for the SPL Token program.
    processor.add_builtin(
        &loader,
        solana_sdk::bpf_loader::id(),
        "solana_bpf_loader_program",
        ProgramCacheEntry::new_builtin(
            0,
            b"solana_bpf_loader_program".len(),
            solana_bpf_loader_program::Entrypoint::vm,
        ),
    );

    /*
    let svm = transaction_processor::TransactionBatchProcessor::new(
        *slot as u64,
        block.block_time.unwrap() as u64,
        HashSet::new(),
    );
    */
}
