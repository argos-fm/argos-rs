use std::{
    error::Error,
    fs::File,
    io::{BufWriter, Seek, SeekFrom, Write},
    str::FromStr,
};

use solana_client::rpc_client::{self, GetConfirmedSignaturesForAddress2Config, RpcClient};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    loader_upgradeable_instruction::UpgradeableLoaderInstruction, pubkey::Pubkey,
    signature::Signature,
};
use solana_transaction_status::{option_serializer::OptionSerializer, UiTransactionEncoding};

// 1 get buffers and upgrade slots
// 2 dl buffers from slot to slot ?

fn main() -> Result<(), Box<dyn Error>> {
    let ore = Pubkey::from_str("oreV2ZymfyeXgNgBdqMkumTqqAprVqgBWQfoYkrtKWQ")?;
    let client = rpc_client::RpcClient::new(
        "https://mainnet.helius-rpc.com/?api-key=00aaba96-cf5f-40ed-9555-7cafc5a3d85c".to_string(),
    );

    let versions = get_versions(&client, &ore)?;
    for v in versions.chunks(2) {
        let current = &v[0];
        let prev_signatue = v.get(1).map(|v| v.signature);
        download_version(&client, &ore, current, prev_signatue)?;
    }

    Ok(())
}

#[derive(Debug)]
struct Version {
    signature: Signature,
    slot: u64,
    buffer: Pubkey,
}
fn get_versions(client: &RpcClient, program_id: &Pubkey) -> Result<Vec<Version>, Box<dyn Error>> {
    let mut versions = vec![];
    let program_data_id =
        solana_program::bpf_loader_upgradeable::get_program_data_address(&program_id);
    let sigs = client.get_signatures_for_address(&program_data_id);
    if let Ok(signatures) = sigs {
        for sig in signatures {
            if sig.err.is_some() {
                continue;
            }
            let slot = sig.slot;
            let date = time::OffsetDateTime::from_unix_timestamp(sig.block_time.unwrap());
            // println!("{:?} {:?} {}", sig.signature, date, sig.slot);
            let signature = Signature::from_str(&sig.signature)?;
            let tx = client.get_transaction(&signature, UiTransactionEncoding::Base64)?;
            let decoded = tx.transaction.transaction.decode().unwrap();
            let ixs = decoded.message.instructions();
            let accounts = decoded.message.static_account_keys();

            for ix in ixs {
                let program_id = accounts[ix.program_id_index as usize];
                if program_id == solana_program::bpf_loader_upgradeable::ID {
                    let instr: UpgradeableLoaderInstruction = bincode::deserialize(&ix.data)?;
                    if let UpgradeableLoaderInstruction::Upgrade = instr {
                        let program_data = accounts[ix.accounts[0] as usize];
                        let program = accounts[ix.accounts[1] as usize];
                        let buffer = accounts[ix.accounts[2] as usize];
                        let spill_account = accounts[ix.accounts[3] as usize];
                        let rent = accounts[ix.accounts[4] as usize];
                        let clock = accounts[ix.accounts[5] as usize];
                        let authority = accounts[ix.accounts[6] as usize];
                        //println!("Upgrade {:?} {:?} {:?}", program_data, buffer, program);
                        versions.push(Version {
                            signature,
                            slot,
                            buffer,
                        });
                    } else if let UpgradeableLoaderInstruction::DeployWithMaxDataLen {
                        max_data_len,
                    } = instr
                    {
                        let payer = accounts[ix.accounts[0] as usize];
                        let program_data = accounts[ix.accounts[1] as usize];
                        let program = accounts[ix.accounts[2] as usize];
                        let buffer = accounts[ix.accounts[3] as usize];
                        let rent = accounts[ix.accounts[4] as usize];
                        let clock = accounts[ix.accounts[5] as usize];
                        let authority = accounts[ix.accounts[6] as usize];
                        //println!(
                        //    "DeployWithMaxDataLen {:?} {:?} {:?}",
                        //    program_data, buffer, program
                        //);
                        versions.push(Version {
                            signature,
                            slot,
                            buffer,
                        });
                    } else {
                        //println!("Instuction {:?}", instr);
                    }
                } else {
                    //println!("Wrong program Id {:?}", program_id);
                }
            }

            if let OptionSerializer::Some(inner_ixs) =
                tx.transaction.meta.unwrap().inner_instructions
            {
                for inner_ix in inner_ixs {
                    let instructions = inner_ix.instructions;
                    for ix in instructions {
                        let (program_id_index, ix_accounts, data) = match ix {
                            solana_transaction_status::UiInstruction::Compiled(
                                ui_compiled_instruction,
                            ) => (
                                ui_compiled_instruction.program_id_index,
                                ui_compiled_instruction.accounts,
                                ui_compiled_instruction.data,
                            ),
                            solana_transaction_status::UiInstruction::Parsed(_) => todo!(),
                        };
                        let program_id =
                            decoded.message.static_account_keys()[program_id_index as usize];
                        if program_id == solana_program::bpf_loader_upgradeable::ID {
                            let data_hex = bs58::decode(data).into_vec().unwrap();
                            let instr: UpgradeableLoaderInstruction =
                                bincode::deserialize(&data_hex)?;
                            if let UpgradeableLoaderInstruction::Upgrade = instr {
                                let program_data = accounts[ix_accounts[0] as usize];
                                let program = accounts[ix_accounts[1] as usize];
                                let buffer = accounts[ix_accounts[2] as usize];
                                let spill_account = accounts[ix_accounts[3] as usize];
                                let rent = accounts[ix_accounts[4] as usize];
                                let clock = accounts[ix_accounts[5] as usize];
                                let authority = accounts[ix_accounts[6] as usize];
                                //println!("Upgrade {:?} {:?} {:?}", program_data, buffer, program);
                                versions.push(Version {
                                    signature,
                                    slot,
                                    buffer,
                                });
                            } else if let UpgradeableLoaderInstruction::DeployWithMaxDataLen {
                                max_data_len,
                            } = instr
                            {
                                let payer = accounts[ix_accounts[0] as usize];
                                let program_data = accounts[ix_accounts[1] as usize];
                                let program = accounts[ix_accounts[2] as usize];
                                let buffer = accounts[ix_accounts[3] as usize];
                                let rent = accounts[ix_accounts[4] as usize];
                                let clock = accounts[ix_accounts[5] as usize];
                                let authority = accounts[ix_accounts[6] as usize];
                                //println!(
                                //    "DeployWithMaxDataLen {:?} {:?} {:?}",
                                //    program_data, buffer, program
                                //);
                                versions.push(Version {
                                    signature,
                                    slot,
                                    buffer,
                                });
                            } else {
                                //println!("[{}] Instuction {:?}", inner_ix.index, instr);
                            }
                        } else {
                            //println!("[{}] Wrong program Id {:?}", inner_ix.index, program_id);
                        }
                    }
                }
            }
        }
    }
    Ok(versions)
}

fn download_version(
    client: &RpcClient,
    program_id: &Pubkey,
    version: &Version,
    prev_signatue: Option<Signature>,
) -> Result<(), Box<dyn Error>> {
    let file_path = format!("{}-{}.so", program_id.to_string(), version.slot);

    // Open the file and wrap it with BufWriter for performance
    let file = File::create(file_path)?;
    let mut writer = BufWriter::new(file);

    let signatures = client.get_signatures_for_address_with_config(
        &version.buffer,
        GetConfirmedSignaturesForAddress2Config {
            before: Some(version.signature),
            until: prev_signatue,
            limit: Some(1000),
            commitment: Some(CommitmentConfig::finalized()),
        },
    )?;
    for sig in signatures {
        let signature = Signature::from_str(&sig.signature)?;
        let tx = client.get_transaction(&signature, UiTransactionEncoding::Base64)?;
        let decoded = tx.transaction.transaction.decode().unwrap();
        let accounts = decoded.message.static_account_keys();
        for ix in decoded.message.instructions() {
            if ix.program_id(&accounts) == &solana_program::bpf_loader_upgradeable::ID {
                let instr: UpgradeableLoaderInstruction = bincode::deserialize(&ix.data)?;
                if let UpgradeableLoaderInstruction::Write { offset, bytes } = instr {
                    println!("[+] write {offset}, {}", bytes.len());
                    writer.seek(SeekFrom::Start(offset as u64))?;
                    writer.write_all(&bytes)?;
                } else {
                    println!("Not a write : {:?}", instr);
                }
            } else {
                println!("Wrong program id {} ", ix.program_id(&accounts));
            }
        }
    }

    writer.flush()?;
    Ok(())
}
