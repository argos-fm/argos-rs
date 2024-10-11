use std::{collections::HashSet, error::Error, fs::File, io::Read, str::FromStr};

use solana_client::{rpc_client, rpc_config::RpcTransactionConfig};
use solana_program_runtime::loaded_programs::{BlockRelation, ForkGraph};
use solana_sdk::{
    account::AccountSharedData,
    clock::{Slot, GENESIS_EPOCH},
    message::{v0::LoadedAddresses, AddressLoader},
    pubkey::Pubkey,
    reserved_account_keys::ReservedAccountKeys,
    signature::Signature,
    transaction::{MessageHash, SanitizedTransaction, Transaction, VersionedTransaction},
};
use solana_svm::{
    transaction_processing_callback::TransactionProcessingCallback,
    transaction_processor::{
        self, TransactionBatchProcessor, TransactionProcessingConfig,
        TransactionProcessingEnvironment,
    },
};
use solana_transaction_status::UiTransactionEncoding;

pub struct MockForkGraph {}

impl ForkGraph for MockForkGraph {
    fn relationship(&self, a: Slot, b: Slot) -> BlockRelation {
        match a.cmp(&b) {
            std::cmp::Ordering::Less => BlockRelation::Ancestor,
            std::cmp::Ordering::Equal => BlockRelation::Equal,
            std::cmp::Ordering::Greater => BlockRelation::Descendant,
        }
    }
}

#[derive(Clone)]
pub struct MockBankCallback {}

impl TransactionProcessingCallback for MockBankCallback {
    fn account_matches_owners(&self, account: &Pubkey, owners: &[Pubkey]) -> Option<usize> {
        println!("[+] account_matches_owners {} , {:?}", account, owners);
        None
    }

    fn get_account_shared_data(&self, pubkey: &Pubkey) -> Option<AccountSharedData> {
        println!("[+] get_account_shared_data {}", pubkey);
        None
    }

    fn add_builtin_account(&self, name: &str, program_id: &Pubkey) {
        println!("[+] add_builtin_account {} {}", name, program_id);
    }
}

#[derive(Clone)]
struct MockLoader {}
impl AddressLoader for MockLoader {
    fn load_addresses(
        self,
        lookups: &[solana_sdk::message::v0::MessageAddressTableLookup],
    ) -> Result<solana_sdk::message::v0::LoadedAddresses, solana_sdk::message::AddressLoaderError>
    {
        println!("[+] load_addresses {:?}", lookups);
        // faked
        Ok(LoadedAddresses {
            writable: vec![],
            readonly: vec![],
        })
    }
}

// 2Fy56jti4eJhcmBTS3Y4EzL72cu739xSZMTBugppfWmHHxGKLr63CL2E6jq3cGgtPyTnfFJMNvEYBoRppt3qCDPZ
const TEST_TX: &[u8] = &[1, 62, 236, 122, 110, 76, 5, 129, 201, 229, 32, 217, 63, 224, 55, 6, 153, 140, 207, 197, 109, 40, 37, 42, 26, 11, 21, 10, 9, 132, 232, 88, 28, 31, 121, 119, 45, 162, 83, 32, 158, 154, 134, 105, 173, 72, 253, 201, 88, 102, 172, 85, 44, 121, 102, 250, 113, 167, 226, 164, 10, 101, 193, 4, 4, 128, 1, 0, 8, 16, 14, 41, 126, 213, 31, 216, 10, 173, 222, 41, 31, 113, 64, 26, 161, 127, 224, 84, 118, 138, 220, 186, 113, 196, 161, 174, 182, 55, 212, 99, 164, 162, 23, 171, 177, 79, 38, 181, 72, 127, 82, 246, 121, 183, 58, 148, 37, 247, 111, 227, 55, 49, 148, 118, 133, 47, 190, 242, 27, 25, 250, 73, 15, 39, 46, 114, 87, 141, 32, 199, 225, 159, 144, 220, 206, 49, 171, 70, 151, 98, 6, 190, 127, 134, 95, 52, 250, 62, 32, 170, 108, 73, 142, 226, 134, 202, 121, 197, 209, 65, 92, 101, 214, 80, 156, 196, 229, 29, 54, 60, 31, 26, 51, 20, 251, 9, 165, 30, 108, 139, 32, 51, 169, 40, 246, 191, 30, 176, 184, 166, 243, 188, 147, 176, 164, 103, 137, 42, 168, 175, 5, 67, 229, 124, 228, 208, 143, 161, 26, 3, 24, 53, 189, 188, 176, 182, 95, 136, 97, 144, 189, 6, 44, 205, 200, 135, 145, 193, 192, 135, 66, 104, 238, 86, 62, 105, 31, 58, 141, 189, 122, 198, 47, 29, 163, 163, 47, 250, 120, 212, 38, 137, 194, 55, 255, 113, 110, 66, 8, 9, 113, 242, 198, 157, 212, 208, 22, 167, 197, 122, 174, 113, 39, 4, 192, 110, 51, 51, 43, 221, 96, 87, 255, 9, 212, 44, 46, 187, 237, 119, 61, 187, 90, 0, 172, 7, 205, 119, 11, 124, 110, 71, 80, 89, 186, 21, 248, 27, 4, 37, 141, 214, 129, 197, 198, 120, 3, 6, 70, 111, 229, 33, 23, 50, 255, 236, 173, 186, 114, 195, 155, 231, 188, 140, 229, 187, 197, 247, 18, 107, 44, 67, 155, 58, 64, 0, 0, 0, 6, 167, 213, 23, 24, 123, 209, 102, 53, 218, 212, 4, 85, 253, 194, 192, 193, 36, 198, 143, 33, 86, 117, 165, 219, 186, 203, 95, 8, 0, 0, 0, 6, 167, 213, 23, 25, 47, 10, 175, 198, 242, 101, 227, 251, 119, 204, 122, 218, 130, 197, 41, 208, 190, 59, 19, 110, 45, 0, 85, 32, 0, 0, 0, 6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237, 95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169, 11, 188, 15, 182, 203, 29, 221, 28, 227, 242, 242, 171, 26, 14, 188, 177, 157, 107, 138, 3, 18, 82, 116, 20, 91, 31, 128, 139, 185, 154, 240, 91, 12, 0, 219, 150, 196, 7, 68, 52, 57, 150, 226, 76, 65, 22, 97, 247, 67, 207, 209, 20, 73, 209, 156, 182, 251, 71, 254, 247, 153, 202, 236, 128, 63, 113, 173, 117, 172, 167, 151, 196, 70, 147, 147, 48, 200, 107, 10, 150, 153, 74, 95, 30, 153, 120, 10, 217, 145, 56, 147, 241, 81, 157, 218, 48, 140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153, 218, 255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89, 109, 74, 73, 184, 147, 104, 22, 250, 143, 93, 55, 225, 70, 249, 155, 75, 249, 254, 7, 237, 169, 199, 116, 202, 120, 16, 252, 21, 17, 255, 123, 215, 6, 8, 0, 5, 2, 224, 200, 16, 0, 8, 0, 9, 3, 112, 23, 0, 0, 0, 0, 0, 0, 12, 0, 32, 184, 166, 243, 188, 147, 176, 164, 103, 137, 42, 168, 175, 5, 67, 229, 124, 228, 208, 143, 161, 26, 3, 24, 53, 189, 188, 176, 182, 95, 136, 97, 144, 12, 0, 32, 121, 197, 209, 65, 92, 101, 214, 80, 156, 196, 229, 29, 54, 60, 31, 26, 51, 20, 251, 9, 165, 30, 108, 139, 32, 51, 169, 40, 246, 191, 30, 176, 13, 24, 0, 6, 1, 21, 14, 2, 37, 36, 11, 15, 38, 4, 9, 10, 40, 35, 30, 22, 16, 24, 27, 28, 20, 23, 45, 59, 22, 178, 213, 139, 197, 160, 196, 15, 0, 0, 0, 186, 111, 134, 167, 244, 32, 86, 243, 230, 41, 232, 168, 38, 119, 233, 243, 246, 134, 161, 252, 147, 193, 117, 163, 23, 0, 0, 0, 0, 0, 0, 0, 0, 13, 24, 0, 5, 1, 31, 14, 7, 41, 42, 11, 15, 43, 3, 9, 10, 39, 32, 19, 26, 18, 33, 29, 17, 34, 25, 45, 59, 22, 178, 213, 139, 197, 160, 196, 0, 0, 0, 0, 186, 111, 134, 167, 244, 32, 86, 243, 230, 41, 232, 168, 38, 119, 233, 243, 246, 134, 161, 252, 147, 193, 117, 163, 23, 0, 0, 0, 0, 0, 0, 0, 0, 1, 34, 91, 231, 129, 50, 247, 195, 57, 75, 248, 242, 78, 136, 217, 199, 161, 52, 141, 242, 149, 89, 70, 163, 147, 234, 146, 52, 19, 126, 165, 165, 49, 20, 18, 29, 26, 24, 22, 9, 17, 23, 19, 31, 25, 20, 21, 28, 16, 15, 14, 27, 30, 8, 8, 4, 6, 7, 11, 5, 12, 10, 13];

fn main() -> Result<(), Box<dyn Error>> {
    let client = rpc_client::RpcClient::new(
        "https://mainnet.helius-rpc.com/?api-key=00aaba96-cf5f-40ed-9555-7cafc5a3d85c".to_string(),
    );

    /* 
    let signature = Signature::from_str(
        "2Fy56jti4eJhcmBTS3Y4EzL72cu739xSZMTBugppfWmHHxGKLr63CL2E6jq3cGgtPyTnfFJMNvEYBoRppt3qCDPZ",
    )?;
    let res_tx = client.get_transaction_with_config(
        &signature,
        RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: None,
            max_supported_transaction_version: Some(0),
        },
    )?;

    let tx = res_tx.transaction.transaction.decode().unwrap();
    let bytes = bincode::serialize(&tx)?;
    println!("{:?}", bytes);
    */
    let tx: VersionedTransaction = bincode::deserialize(TEST_TX)?;

    let slot = 0;
    let epoch = GENESIS_EPOCH;
    let builtin_program_ids = HashSet::new();

    let processor: TransactionBatchProcessor<MockForkGraph> =
        transaction_processor::TransactionBatchProcessor::new(slot, epoch, builtin_program_ids);

    println!("[+] TX ok");
    let sanetized_tx = SanitizedTransaction::try_create(
        tx,
        MessageHash::Compute,
        None,
        MockLoader {},
        &ReservedAccountKeys::default().active,
    )?;

    let callbacks = MockBankCallback {};
    let check_results = vec![];
    let environment = TransactionProcessingEnvironment::default();
    let config = TransactionProcessingConfig::default();

    let res = processor.load_and_execute_sanitized_transactions(
        &callbacks,
        &[sanetized_tx],
        check_results,
        &environment,
        &config,
    );

    println!(" error_metrics : {:?}\n execute_timings:{:?}\n execution_results:{:?}\nloaded_transactions: {:?}", res.error_metrics, res.execute_timings, res.execution_results, res.loaded_transactions);

    // load program
    // let mut f = File::open("oreV2ZymfyeXgNgBdqMkumTqqAprVqgBWQfoYkrtKWQ-280869624.so")?;
    // let mut data: Vec<_> = vec![];
    // f.read_to_end(&mut data)?;

    // call it with rand args

    Ok(())
}
