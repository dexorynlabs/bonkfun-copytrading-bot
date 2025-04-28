use anchor_client::solana_client::rpc_client::RpcClient;
use anchor_client::solana_sdk::{
    hash::Hash,
    instruction::Instruction,
    signature::Keypair,
    signer::Signer,
    system_instruction, system_transaction,
    transaction::{Transaction, VersionedTransaction},
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use colored::Colorize;
use spl_token::ui_amount_to_amount;
use std::{env, str::FromStr};
use std::{sync::Arc, time::Duration};

use tokio::time::Instant;

use crate::common::config::{create_nonblocking_rpc_client, Config};
// use crate::services::bloxroute::{self, BloxRouteClient};
use crate::{
    common::logger::Logger,
    services::{
        jito::{self, JitoClient},
        nozomi,
        zeroslot::{self, ZeroSlotClient},
    },
};

// pub async fn new_signed_and_send_normal(
//     recent_blockhash: solana_sdk::hash::Hash,
//     keypair: &Keypair,
//     mut instructions: Vec<Instruction>,
//     logger: &Logger,
// ) -> Result<Vec<String>> {
//     let start_time = Instant::now();
//     let mut txs = vec![];
//     // ADD Priority fee
//     // -------------
//     let unit_limit = get_unit_limit();
//     let unit_price = get_unit_price();

//     let modify_compute_units =
//         solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(unit_limit);
//     let add_priority_fee =
//         solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(unit_price);
//     instructions.insert(1, modify_compute_units);
//     instructions.insert(2, add_priority_fee);
//     // send init tx
//     let txn = Transaction::new_signed_with_payer(
//         &instructions,
//         Some(&keypair.pubkey()),
//         &vec![keypair],
//         recent_blockhash,
//     );
//     let config = Config::get().await;
//     // let rpc_client = &config.app_state.rpc_nonblocking_client;
//     let rpc_nonblocking_client = create_nonblocking_rpc_client().await.unwrap();
//     let sig = rpc_nonblocking_client.send_and_confirm_transaction(&txn).await?;
//     txs.push(sig.to_string());

//     logger.log(
//         format!("[TXN-ELLAPSED(NORMAL)]: {:?}", start_time.elapsed())
//             .yellow()
//             .to_string(),
//     );

//     Ok(txs)
// }

#[derive(Clone)]
pub struct TxnForm {
    pub title: String,
    pub timestamp: Duration,
    pub txn_hash: String,
}

fn sort_txn_forms_by_timestamp(txn_forms: &mut Vec<TxnForm>) {
    txn_forms.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
}

pub async fn new_signed_and_send(
    recent_blockhash: anchor_client::solana_sdk::hash::Hash,
    keypair: &Keypair,
    mut instructions: Vec<Instruction>,
    logger: &Logger,
    start_time: Instant,
) -> Result<Vec<TxnForm>> {
    // logger.log(
    //     format!("[TXN-BEGIN(JITO)]: {:?} => {:?}", start_time.elapsed(), Utc::now())
    //         .yellow()
    //         .to_string(),
    // );
    let (tip_account, tip1_account) = jito::get_tip_account()?;

    // jito tip, the upper limit is 0.1
    let tip = jito::get_tip_value().await?;
    let fee = jito::get_priority_fee().await?;
    let tip_lamports = ui_amount_to_amount(tip, spl_token::native_mint::DECIMALS);
    let fee_lamports = ui_amount_to_amount(fee, spl_token::native_mint::DECIMALS);

    let jito_tip_instruction =
        system_instruction::transfer(&keypair.pubkey(), &tip_account, tip_lamports);
    let jito_tip2_instruction =
        system_instruction::transfer(&keypair.pubkey(), &tip1_account, fee_lamports);
    instructions.push(jito_tip_instruction);
    instructions.push(jito_tip2_instruction);

    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &vec![keypair],
        recent_blockhash,
    );

    // let simulate_result = client.simulate_transaction(&txn)?;
    // logger.log("Tx Stimulate".to_string());
    // if let Some(logs) = simulate_result.value.logs {
    //     for log in logs {
    //         logger.log(log.to_string());
    //     }
    // }
    // if let Some(err) = simulate_result.value.err {
    //     return Err(anyhow::anyhow!("{}", err));
    // };

    let jito_client = Arc::new(JitoClient::new(
        format!("{}/api/v1/transactions", *jito::BLOCK_ENGINE_URL).as_str(),
    ));
    let sig = match jito_client.send_transaction(&txn).await {
        Ok(signature) => signature,
        Err(e) => {
            return Err(anyhow::anyhow!(format!(
                "[JITO] => send_transaction status get timeout: {}",
                e
            )
            .red()
            .italic()
            .to_string()));
        }
    };

    // logger.log(
    //     format!("[TXN-ELLAPSED(JITO)]: {:?}", start_time.elapsed())
    //         .yellow()
    //         .to_string(),
    // );
    Ok(vec![TxnForm {
        title: "jito".to_string(),
        timestamp: start_time.elapsed(),
        txn_hash: sig.clone().to_string(),
    }])
}

pub async fn new_signed_and_send_zeroslot(
    recent_blockhash: anchor_client::solana_sdk::hash::Hash,
    keypair: &Keypair,
    mut instructions: Vec<Instruction>,
    logger: &Logger,
    start_time: Instant,
) -> Result<Vec<TxnForm>> {
    let tip_account = zeroslot::get_tip_account()?;

    // zeroslot tip, the upper limit is 0.1
    let tip = zeroslot::get_tip_value().await?;
    let tip_lamports = ui_amount_to_amount(tip, spl_token::native_mint::DECIMALS);

    let zeroslot_tip_instruction =
        system_instruction::transfer(&keypair.pubkey(), &tip_account, tip_lamports);
    instructions.insert(0, zeroslot_tip_instruction);

    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &vec![keypair],
        recent_blockhash,
    );

    // let simulate_result = client.simulate_transaction(&txn)?;
    // logger.log("Tx Stimulate".to_string());
    // if let Some(logs) = simulate_result.value.logs {
    //     for log in logs {
    //         logger.log(log.to_string());
    //     }
    // }
    // if let Some(err) = simulate_result.value.err {
    //     return Err(anyhow::anyhow!("{}", err));
    // };

    let zeroslot_client = Arc::new(ZeroSlotClient::new((*zeroslot::ZERO_SLOT_URL).as_str()));
    let sig = match zeroslot_client.send_transaction(&txn).await {
        Ok(signature) => signature,
        Err(e) => {
            return Err(anyhow::anyhow!(format!(
                "[ZEROSLOT] => send_transaction status get timeout: {}",
                e
            )
            .red()
            .italic()
            .to_string()));
        }
    };

    // logger.log(
    //     format!("[TXN-ELLAPSED]: {:?}", start_time.elapsed())
    //         .yellow()
    //         .to_string(),
    // );
    Ok(vec![TxnForm {
        title: "zeroslot".to_string(),
        timestamp: start_time.elapsed(),
        txn_hash: sig.clone().to_string(),
    }])
}

// prioritization fee = UNIT_PRICE * UNIT_LIMIT
fn get_unit_price() -> u64 {
    env::var("UNIT_PRICE")
        .ok()
        .and_then(|v| u64::from_str(&v).ok())
        .unwrap_or(20000)
}

fn get_unit_limit() -> u32 {
    env::var("UNIT_LIMIT")
        .ok()
        .and_then(|v| u32::from_str(&v).ok())
        .unwrap_or(200_000)
}

pub async fn new_signed_and_send_nozomi(
    recent_blockhash: anchor_client::solana_sdk::hash::Hash,
    keypair: &Keypair,
    mut instructions: Vec<Instruction>,
    logger: &Logger,
    start_time: Instant,
) -> Result<Vec<TxnForm>> {
    // logger.log(
    //     format!("[TXN-BEGIN(NOZOMI)]: {:?} => {:?}", start_time.elapsed(), Utc::now())
    //         .yellow()
    //         .to_string(),
    // );
    let tip_account = nozomi::get_tip_account()?;

    // nozomi tip, the upper limit is 0.1
    let tip = nozomi::get_tip_value().await?;
    let tip_lamports = ui_amount_to_amount(tip, spl_token::native_mint::DECIMALS);

    let nozomi_tip_instruction =
        system_instruction::transfer(&keypair.pubkey(), &tip_account, tip_lamports);
    instructions.insert(0, nozomi_tip_instruction);

    // ADD Priority fee
    // -------------
    let unit_limit = get_unit_limit();
    let unit_price = get_unit_price();

    let modify_compute_units =
        anchor_client::solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(
            unit_limit,
        );
    let add_priority_fee =
        anchor_client::solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
            unit_price,
        );
    instructions.insert(1, modify_compute_units);
    instructions.insert(2, add_priority_fee);

    // send init tx
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &vec![keypair],
        recent_blockhash,
    );

    // let simulate_result = client.simulate_transaction(&txn)?;
    // logger.log("Tx Stimulate".to_string());
    // if let Some(logs) = simulate_result.value.logs {
    //     for log in logs {
    //         logger.log(log.to_string());
    //     }
    // }
    // if let Some(err) = simulate_result.value.err {
    //     return Err(anyhow::anyhow!("{}", err));
    // };

    let zeroslot_client = Arc::new(ZeroSlotClient::new((*nozomi::NOZOMI_URL).as_str()));
    let sig = match zeroslot_client.send_transaction(&txn).await {
        Ok(signature) => signature,
        Err(e) => {
            return Err(anyhow::anyhow!(format!(
                "[NOZOMI] => send_transaction status get timeout: {}",
                e
            )
            .red()
            .italic()
            .to_string()));
        }
    };
    // logger.log(
    //     format!("[TXN-ELLAPSED(NOZOMI)]: {:?}", start_time.elapsed())
    //         .yellow()
    //         .to_string(),
    // );

    Ok(vec![TxnForm {
        title: "nozomi".to_string(),
        timestamp: start_time.elapsed(),
        txn_hash: sig.clone().to_string(),
    }])
}

// pub async fn new_signed_and_send_bloxroute(
//     recent_blockhash: solana_sdk::hash::Hash,
//     keypair: &std::sync::Arc<Keypair>,
//     mut instructions: Vec<Instruction>,
//     logger: &Logger,
//     start_time: Instant,
// ) -> Result<Vec<TxnForm>> {
//     // logger.log(
//     //     format!("[TXN-BEGIN(BLOXROUTE)]: {:?} => {:?}", start_time.elapsed(), Utc::now())
//     //         .yellow()
//     //         .to_string(),
//     // );
//     let tip_account = bloxroute::get_tip_account()?;

//     // bloxroute tip, the upper limit is 0.1
//     let tip = bloxroute::get_tip_value().await?;
//     let tip_lamports = ui_amount_to_amount(tip, spl_token::native_mint::DECIMALS);

//     let bloxroute_tip_instruction =
//         system_instruction::transfer(&keypair.pubkey(), &tip_account, tip_lamports);
//     instructions.insert(0, bloxroute_tip_instruction);

//     // ADD Priority fee
//     // -------------
//     let unit_limit = get_unit_limit();
//     let unit_price = get_unit_price();

//     let modify_compute_units =
//         solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(unit_limit);
//     let add_priority_fee =
//         solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(unit_price);
//     instructions.insert(1, modify_compute_units);
//     instructions.insert(2, add_priority_fee);

//     // send init tx
//     let txn = Transaction::new_signed_with_payer(
//         &instructions,
//         Some(&keypair.pubkey()),
//         &vec![keypair],
//         recent_blockhash,
//     );

//     let mut bloxroute_client = BloxRouteClient::new().await?;
//     let sig = match bloxroute_client.send_transaction(&txn).await {
//         Ok(signature) => signature,
//         Err(e) => {
//             return Err(anyhow::anyhow!(format!(
//                 "[BLOXROUTE] => send_transaction status get timeout: {}",
//                 e
//             )
//             .red()
//             .italic()
//             .to_string()));
//         }
//     };
//     // logger.log(
//     //     format!("[TXN-ELLAPSED(BLOXROUTE)]: {:?}", start_time.elapsed())
//     //         .yellow()
//     //         .to_string(),
//     // );

//     Ok(vec![TxnForm {
//         title: "bloxroute".to_string(),
//         timestamp: start_time.elapsed(),
//         txn_hash: sig[0].clone(),
//     }])
// }

pub async fn new_signed_and_send_spam(
    recent_blockhash: anchor_client::solana_sdk::hash::Hash,
    keypair: &std::sync::Arc<Keypair>,
    instructions: Vec<Instruction>,
    logger: &Logger,
    start_time: Instant,
) -> Result<Vec<TxnForm>> {
    // Assuming keypair is already defined as Arc<Keypair>
    let logger_clone = logger.clone();
    let keypair_clone = Arc::clone(&keypair); // Clone the Arc for the first future

    let logger_clone1 = logger.clone();
    let keypair_clone1 = Arc::clone(&keypair); // Clone the Arc for the second future

    // let logger_clone2 = logger.clone();
    // let keypair_clone2 = Arc::clone(&keypair); // Clone the Arc for the second future

    // let logger_clone3 = logger.clone();
    // let keypair_clone3 = Arc::clone(&keypair); // Clone the Arc for the second future

    // Clone instructions if necessary
    let instructions_clone_for_jito = instructions.clone();
    let instructions_clone_for_nozomi = instructions.clone();
    // let instructions_clone_for_zeroslot = instructions.clone();
    // let instructions_clone_for_bloxroute = instructions.clone();

    // Create the futures for both transaction sending methods
    let jito_handle = tokio::spawn(async move {
        new_signed_and_send(
            recent_blockhash,
            &keypair_clone,
            instructions_clone_for_jito,
            &logger_clone,
            start_time,
        )
        .await
    });

    let nozomi_handle = tokio::spawn(async move {
        new_signed_and_send_nozomi(
            recent_blockhash,
            &keypair_clone1,
            instructions_clone_for_nozomi,
            &logger_clone1,
            start_time,
        )
        .await
    });

    // let zeroslot_handle = tokio::spawn(async move {
    //     new_signed_and_send_zeroslot(
    //         recent_blockhash,
    //         &keypair_clone2,
    //         instructions_clone_for_zeroslot,
    //         &logger_clone2,
    //     )
    //     .await
    // });

    // let bloxroute_handle = tokio::spawn(async move {
    //     new_signed_and_send_bloxroute(
    //         recent_blockhash,
    //         &keypair_clone3,
    //         instructions_clone_for_bloxroute,
    //         &logger_clone3,
    //         start_time,
    //     )
    //     .await
    // });

    let mut successful_results = Vec::new();

    let (jito_result, nozomi_result) = match tokio::try_join!(jito_handle, nozomi_handle) {
        Ok((jito_result, nozomi_result)) => {
            let jito_result = jito_result?;
            let nozomi_result = nozomi_result?;
            (jito_result, nozomi_result)
        }
        Err(err) => {
            logger.log(format!("Failed with {}, ", err).red().to_string());
            return Err(anyhow!(format!("{}", err)));
        }
    };
    successful_results.push(jito_result[0].clone());
    successful_results.push(nozomi_result[0].clone());
    // successful_results.push(bloxroute_result[0].clone());

    // Await both futures
    // let results = futures::future::join_all(vec![jito_future, nozomi_future, bloxroute_future]).await;

    // let mut errors: Vec<String> = Vec::new();

    // for result in results {
    //     match result {
    //         Ok(Ok(res)) => {
    //             successful_results.push(res);
    //         }, // Push if success
    //         Ok(Err(e)) => errors.push(e.to_string()), // Collect error message
    //         Err(e) => errors.push(format!("Task failed: {:?}", e)), // Collect task failure
    //     }
    // }

    // If there are any errors, print them
    // if !errors.is_empty() {
    //     for error in &errors {
    //         eprintln!("{}", error); // Print errors to stderr
    //     }

    //     // If no successful results were collected, return an error
    //     if successful_results.is_empty() {
    //         return Err(anyhow::anyhow!(format!("All tasks failed with these errors: {:?}", errors)));
    //     }
    // }

    sort_txn_forms_by_timestamp(&mut successful_results);

    Ok(successful_results)
}
