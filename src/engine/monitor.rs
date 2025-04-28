use anchor_client::solana_sdk::signer::Signer;
use anchor_client::solana_sdk::{hash::Hash, pubkey::Pubkey, signature::Signature};
use anchor_lang::pubkey;
use maplit::hashmap;
use spl_token::solana_program::native_token::sol_to_lamports;
use std::collections::HashMap;
use std::str::FromStr;

use crate::common::config::{
    AppState, SwapConfig, JUPITER_PROGRAM, OKX_DEX_PROGRAM, PUMP_SWAP_BUY_INSTRUCTION, PUMP_SWAP_SELL_INSTRUCTION
};
use crate::common::constants::{PUMPSWAP_FEE_ACCOUNTS, PUMP_AMM_PROGRAM, WSOL};
use crate::common::{logger::Logger, targetlist::Targetlist};
use crate::core::token::amount_to_lamports;
use crate::core::tx::new_signed_and_send;
use crate::dex::pump_swap::{buy_instruction, sell_instruction, BuyOption, PoolInfo, SellOption};
// use crate::core::cpi::build_pump_swap_ixn_by_cpi;
use anyhow::Result;
use chrono::Utc;
use colored::Colorize;
use futures_util::stream::StreamExt;
use futures_util::SinkExt;
use tokio::time::Instant;
use yellowstone_grpc_client::GeyserGrpcClient;
use yellowstone_grpc_proto::geyser::{
    subscribe_update::UpdateOneof, CommitmentLevel, SubscribeRequest,
    SubscribeRequestFilterTransactions, SubscribeUpdateTransaction,
};
use spl_associated_token_account::get_associated_token_address;

#[derive(Clone, Debug)]
pub struct BondingCurveInfo {
    pub bonding_curve: Pubkey,
    pub new_virtual_sol_reserve: u64,
    pub new_virtual_token_reserve: u64,
}

#[derive(Clone, Debug)]
pub struct TradeInfoFromToken {
    pub slot: u64,
    pub recent_blockhash: Hash,
    pub signature: String,
    pub target: String,
    pub mint: String,
    pub token_amount_list: TokenAmountList,
    pub sol_amount_list: SolAmountList,
    pub pool: String,
    pub decimal: u32
}

#[derive(Clone, Debug)]
pub struct TokenAmountList {
    token_pre_amount: f64,
    token_post_amount: f64,
}

#[derive(Clone, Debug)]
pub struct SolAmountList {
    sol_pre_amount: f64,
    sol_post_amount: f64,
}

pub struct FilterConfig {
    program_ids: Vec<String>,
}

impl TradeInfoFromToken {
    pub fn from_json(txn: SubscribeUpdateTransaction) -> Result<Self> {
        let slot = txn.slot;
        let (recent_blockhash, signature, target, mint, token_amount_list, sol_amount_list, bonding_curve, mint_decimal) =
            if let Some(transaction) = txn.transaction {
                let signature = match Signature::try_from(transaction.signature.clone()) {
                    Ok(signature) => format!("{:?}", signature),
                    Err(_) => "".to_string(),
                };
                let recent_blockhash_slice = &transaction
                    .transaction
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get recent blockhash"))?
                    .message
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get recent blockhash"))?
                    .recent_blockhash;
                let recent_blockhash = Hash::new(recent_blockhash_slice);

                let mut mint = String::new();
                let mut bonding_curve = String::new();
                let mut sol_post_amount = 0_f64;
                let mut sol_pre_amount = 0_f64;
                let mut token_post_amount = 0_f64;
                let mut token_pre_amount = 0_f64;
                let mut mint_decimal = 0_u32;
                // Retrieve Target Wallet Pubkey
                let account_keys = &transaction
                    .transaction
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get account keys"))?
                    .message
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("Failed to get account keys"))?
                    .account_keys;

                let target = Pubkey::try_from(account_keys[0].clone())
                    .unwrap()
                    .to_string();

                if let Some(meta) = transaction.meta.clone() {
                    for post_token_balance in meta.post_token_balances.iter() {
                        let owner = post_token_balance.owner.clone();
                        if PUMPSWAP_FEE_ACCOUNTS.contains(&owner.as_str()) {
                            continue;
                        }
                        if owner.clone() != target {
                            bonding_curve = owner.clone();
                        }
                        if owner == target || owner == bonding_curve {
                            let post_mint = post_token_balance.mint.clone();
                            if post_mint == WSOL {
                                continue;
                            }
                            mint = post_mint;
                        }
                    }
                    if mint.is_empty() {
                        for pre_token_balance in meta.pre_token_balances.iter() {
                            let owner = pre_token_balance.owner.clone();
                            if PUMPSWAP_FEE_ACCOUNTS.contains(&owner.as_str()) {
                                continue;
                            }
                            if owner.clone() != target {
                                bonding_curve = owner.clone();
                            }
                            if owner == target || owner == bonding_curve {
                                let post_mint = pre_token_balance.mint.clone();
                                if post_mint == WSOL {
                                    continue;
                                }
                                mint = post_mint;
                            }
                        }
                    };


                    if mint.is_empty() {
                        return Err(anyhow::anyhow!(format!(
                            "signature[{}]: mint is None",
                            signature
                        )));
                    };

                    if mint == "FTzf8Hh5booHAKSP4axAc9YgfxtbkM7SiJMQjQAppump".to_string() {
                        return Err(anyhow::anyhow!(format!(
                            "He is running volume bot, skip this one!",
                        )));
                    }

                    sol_post_amount = meta
                        .pre_token_balances
                        .iter()
                        .filter_map(|token_balance| {
                            if (token_balance.owner == bonding_curve)
                                && (token_balance.mint == WSOL)
                            {
                                token_balance
                                    .ui_token_amount
                                    .as_ref()
                                    .map(|ui| ui.ui_amount)
                            } else {
                                None
                            }
                        })
                        .next()
                        .unwrap_or(0_f64);

                    sol_pre_amount = meta
                        .post_token_balances
                        .iter()
                        .filter_map(|token_balance| {
                            if (token_balance.owner == bonding_curve)
                                && (token_balance.mint == WSOL)
                            {
                                token_balance
                                    .ui_token_amount
                                    .as_ref()
                                    .map(|ui| ui.ui_amount)
                            } else {
                                None
                            }
                        })
                        .next()
                        .unwrap_or(0_f64);

                    token_post_amount = meta
                        .post_token_balances
                        .iter()
                        .filter_map(|token_balance| {
                            if (token_balance.owner == bonding_curve)
                                && (token_balance.mint == mint)
                            {
                                token_balance
                                    .ui_token_amount
                                    .as_ref()
                                    .map(|ui| ui.ui_amount)
                            } else {
                                None
                            }
                        })
                        .next()
                        .unwrap_or(0_f64);

                    token_pre_amount = meta
                        .pre_token_balances
                        .iter()
                        .filter_map(|token_balance| {
                            if (token_balance.owner == bonding_curve)
                                && (token_balance.mint == mint)
                            {
                                mint_decimal = token_balance
                                    .ui_token_amount
                                    .as_ref()
                                    .map(|ui| ui.decimals).unwrap_or(6_u32);
                                // token_balance.ui_token_amount.as_ref().
                                token_balance
                                    .ui_token_amount
                                    .as_ref()
                                    .map(|ui| ui.ui_amount)
                            } else {
                                None
                            }
                        })
                        .next()
                        .unwrap_or(0_f64);
                }

                let token_amount_list = TokenAmountList {
                    token_pre_amount,
                    token_post_amount,
                };

                let sol_amount_list = SolAmountList {
                    sol_pre_amount,
                    sol_post_amount,
                };

                (
                    recent_blockhash,
                    signature,
                    target,
                    mint,
                    token_amount_list,
                    sol_amount_list,
                    bonding_curve,
                    mint_decimal
                )
            } else {
                return Err(anyhow::anyhow!("Transaction is None"));
            };

        Ok(Self {
            slot,
            recent_blockhash,
            signature,
            target,
            mint,
            token_amount_list,
            sol_amount_list,
            pool: bonding_curve,
            decimal: mint_decimal
        })
    }
}

pub async fn pumpswap_trader(
    yellowstone_grpc_http: String,
    yellowstone_grpc_token: String,
    app_state: AppState,
    swap_config: SwapConfig,
    _targetlist: Targetlist,
) -> Result<(), String> {
    // INITIAL SETTING FOR SUBSCIBE
    // -----------------------------------------------------------------------------------------------------------------------------
    let mut client = GeyserGrpcClient::build_from_shared(yellowstone_grpc_http)
        .map_err(|e| format!("Failed to build client: {}", e))?
        .x_token::<String>(Some(yellowstone_grpc_token))
        .map_err(|e| format!("Failed to set x_token: {}", e))?
        .connect()
        .await
        .map_err(|e| format!("Failed to connect: {}", e))?;

    // HEALTH CHECK
    // -----------------
    // match client.health_check().await {
    //     Ok(health) => {
    //         println!("Health check: {:#?}", health.status);
    //     }
    //     Err(err) => {
    //         println!("Error in Health check: {:#?}", err);
    //     }
    // };

    // SUBSCRIBE
    // -----------------
    loop {
        let logger = Logger::new("[PUMPFUN-AMM-MONITOR] => ".blue().bold().to_string());

        let (mut subscribe_tx, mut stream) = match client.subscribe().await {
            Ok(result) => result,
            Err(e) => {
                logger.log(format!("Failed to subscribe: {}", e).red().to_string());
                // Optionally, you can add a delay here before retrying
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                continue;
            }
        };
    
        let filter_config = FilterConfig {
            program_ids: vec![PUMP_AMM_PROGRAM.to_string()],
        };

        if let Err(e) = subscribe_tx
            .send(SubscribeRequest {
                slots: HashMap::new(),
                accounts: HashMap::new(),
                transactions: hashmap! {
                    "All".to_owned() => SubscribeRequestFilterTransactions {
                        vote: None,
                        failed: Some(false),
                        signature: None,
                        account_include: filter_config.program_ids,
                        account_exclude: vec![JUPITER_PROGRAM.to_string(), OKX_DEX_PROGRAM.to_string()],
                        account_required: Vec::<String>::new()
                    }
                },
                transactions_status: HashMap::new(),
                entry: HashMap::new(),
                blocks: HashMap::new(),
                blocks_meta: HashMap::new(),
                commitment: Some(CommitmentLevel::Processed as i32),
                accounts_data_slice: vec![],
                ping: None,
            })
            .await
        {
            logger.log(
                format!("Failed to send subscribe request: {}", e)
                    .red()
                    .to_string(),
            );
            continue; // Handle the error and reconnect
        }
    
        logger.log("[STARTED. MONITORING]... \n\t".blue().bold().to_string());
    
        // NOW SUBSCRIBE
        // -----------------------------------------------------------------------------------------------------------------------------
        while let Some(message) = stream.next().await {
            match message {
                Ok(msg) => {
                    if let Some(UpdateOneof::Transaction(txn)) = msg.update_oneof {
                        let start_time = Instant::now();
                        if let Some(log_messages) = txn
                            .clone()
                            .transaction
                            .and_then(|txn1| txn1.meta)
                            .map(|meta| meta.log_messages)
                        {
                            let trade_info = match TradeInfoFromToken::from_json(txn.clone()) {
                                Ok(info) => info,
                                Err(_e) => {
                                    logger.log(
                                        format!("Error in parsing txn: {}", _e)
                                            .red()
                                            .italic()
                                            .to_string(),
                                    );
                                    continue;
                                }
                            };
    
                            // CHECK TARGETLIST
                            // -----------------
                            if _targetlist.is_listed_on_target(&trade_info.target) {
                                logger.log(format!(
                                    "\n\t * [TARGETLIST-NOTIFICATION] => (https://solscan.io/tx/{}) \n\t * [SIGNER] => ({}) \n\t * [DETECT] => ({}) \n\t * [TIMESTAMP] => {} :: ({:?}). \n\t",
                                    trade_info.signature,
                                    trade_info.target,
                                    trade_info.mint,
                                    Utc::now(),
                                    start_time.elapsed(),
                                ).yellow().to_string());
                                for log_message in log_messages.iter() {
                                    if log_message.contains(PUMP_SWAP_SELL_INSTRUCTION) || log_message.contains(PUMP_SWAP_BUY_INSTRUCTION) {
                                        //TODO: Add the condition that filters the token.
    
                                        let mut is_buy = true;
                                        let mut trade_string = "BOUGHT";
    
                                        if log_message.contains(PUMP_SWAP_SELL_INSTRUCTION) {
                                            is_buy = false;
                                            trade_string = "SOLD"
                                        }
    
                                        let token_amount = trade_info.token_amount_list.token_pre_amount - trade_info.token_amount_list.token_post_amount;
                                        let sol_amount = trade_info.sol_amount_list.sol_post_amount - trade_info.sol_amount_list.sol_pre_amount;
    
                                        // panic!("This will stop the process with an error message");
                                        // SELL DETECT!
                                        logger.log(format!(
                                            "\n\t * [{}] => (https://solscan.io/tx/{}) - SLOT:({}) \n\t * [MINT] => ({}) \n\t * [TOKEN AMOUNT] => ({}) \n\t * [SOL AMOUNT] => ({}) \n\t * [TIMESTAMP] => {} :: ({:?}). \n\t",
                                            trade_string,
                                            trade_info.signature,
                                            trade_info.slot,
                                            trade_info.mint,
                                            token_amount.abs(),
                                            sol_amount.abs(),
                                            Utc::now(),
                                            start_time.elapsed(),
                                        ).yellow().to_string());
    
                                        let mint = Pubkey::from_str(&trade_info.clone().mint).unwrap();
                                        let wsol_pub = Pubkey::from_str(&WSOL).unwrap();
    
                                        // NOW SELL!
                                        let pool: Pubkey = Pubkey::from_str(&trade_info.clone().pool).unwrap();
                                        let pool_base_token_account: Pubkey = get_associated_token_address(&pool, &mint);
                                        let pool_quote_token_account: Pubkey = get_associated_token_address(&pool, &wsol_pub);
    
                                        let pool_info = PoolInfo {
                                            pool: pool,
                                            pool_base_token_account: pool_base_token_account,
                                            pool_quote_token_account: pool_quote_token_account,
                                        };
    
                                        if !is_buy {
    
                                            let base_amount_in: u64 = amount_to_lamports(token_amount.abs(), trade_info.decimal as u8);
                                            let min_quote_amount_out: u64 = 0;
        
                                            let selloption = SellOption {
                                                base_amount_in,
                                                min_quote_amount_out,
                                            };
        
                                            let ixs = sell_instruction(
                                                &pool_info,
                                                &selloption,
                                                &mint,
                                                &app_state.clone().wallet.pubkey(),
                                            );
        
                                            let recent_blockhash = app_state
                                                .clone()
                                                .rpc_nonblocking_client
                                                .get_latest_blockhash()
                                                .await
                                                .unwrap();
        
                                            let logger = Logger::new(
                                                format!("[AUTO-SELL]() => ").yellow().to_string(),
                                            );
        
                                            let start_time = Instant::now();
        
                                            match new_signed_and_send(
                                                recent_blockhash,
                                                &app_state.clone().wallet,
                                                ixs,
                                                &logger,
                                                start_time.into(),
                                            )
                                            .await
                                            {
                                                Ok(res) => {
                                                    let txn_forms: Vec<String> = res
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(index, txn_form)| {
                                                            format!(
                                                            "{}. [{}]({:?}) => https://solscan.io/tx/{}",
                                                            index + 1,
                                                            txn_form.title,
                                                            txn_form.timestamp,
                                                            txn_form.txn_hash
                                                        )
                                                        })
                                                        .collect();
                                                    let tx_hash_string = txn_forms.join("\n\t\t\t\t");
                                                    logger.log(
                                                        format!(
                                                            "\n\t * [SELL-RESULT] => {} \n\t * [POOL] => ({}) \n\t * [DONE] => {}. \n\t",
                                                            tx_hash_string,
                                                            mint.to_string(),
                                                            Utc::now()
                                                        )
                                                        .green()
                                                        .to_string(),
                                                    );
                                                }
                                                Err(_) => todo!(),
                                            }
                                        } else {
                                            let base_amount_out: u64 = amount_to_lamports(token_amount.abs(), trade_info.decimal as u8);
                                            let max_quote_amount_in: u64 = sol_to_lamports(10.0);
                                            let estimate_wsol_amount = sol_to_lamports(sol_amount.abs() + 0.001);
    
                                            let buyoption = BuyOption {
                                                base_amount_out,
                                                max_quote_amount_in,
                                                estimate_wsol_amount
                                            };
        
                                            let ixs = buy_instruction(
                                                &pool_info,
                                                &buyoption,
                                                &mint,
                                                &app_state.clone().wallet.pubkey(),
                                            );
        
                                            let recent_blockhash = app_state
                                                .clone()
                                                .rpc_nonblocking_client
                                                .get_latest_blockhash()
                                                .await
                                                .unwrap();
        
                                            let logger = Logger::new(
                                                format!("[AUTO-BUY]() => ").yellow().to_string(),
                                            );
        
                                            let start_time = Instant::now();
        
                                            match new_signed_and_send(
                                                recent_blockhash,
                                                &app_state.clone().wallet,
                                                ixs,
                                                &logger,
                                                start_time.into(),
                                            )
                                            .await
                                            {
                                                Ok(res) => {
                                                    let txn_forms: Vec<String> = res
                                                        .iter()
                                                        .enumerate()
                                                        .map(|(index, txn_form)| {
                                                            format!(
                                                            "{}. [{}]({:?}) => https://solscan.io/tx/{}",
                                                            index + 1,
                                                            txn_form.title,
                                                            txn_form.timestamp,
                                                            txn_form.txn_hash
                                                        )
                                                        })
                                                        .collect();
                                                    let tx_hash_string = txn_forms.join("\n\t\t\t\t");
                                                    logger.log(
                                                        format!(
                                                            "\n\t * [BUY-RESULT] => {} \n\t * [POOL] => ({}) \n\t * [DONE] => {}. \n\t",
                                                            tx_hash_string,
                                                            mint.to_string(),
                                                            Utc::now()
                                                        )
                                                        .green()
                                                        .to_string(),
                                                    );
                                                }
                                                Err(_) => todo!(),
                                            }
                                        }
                                        
                                    }
                                }
                            }
                        }
                    }
                }
                Err(error) => {
                    logger.log(
                        format!("Yellowstone gRpc Error: {:?}", error)
                            .red()
                            .to_string(),
                    );
                    break;
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}
