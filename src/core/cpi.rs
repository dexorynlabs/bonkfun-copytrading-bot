use colored::Colorize;
use anchor_client::solana_program::instruction::{AccountMeta, Instruction};
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::{signature::Keypair, signer::Signer};
use spl_associated_token_account::{
    get_associated_token_address,
    instruction::{create_associated_token_account, create_associated_token_account_idempotent},
};
use spl_token::ui_amount_to_amount;
use std::{str::FromStr, sync::Arc};
use tokio::time::Instant;

use crate::{
    common::{
        config::SwapConfig,
        constants::{
            ASSOCIATED_TOKEN_PROGRAM, BONDING_CURVE_SEED, PUMPFUN_EVENT_AUTH, PUMPFUN_FEE_ACC,
            PUMPFUN_GLOBAL, PUMPFUN_PROGRAM_ADDRESS, SYSTEM_PROGRAM, SYSVAR_RENT_PUBKEY,
            TEN_THOUSAND, TOKEN_PROGRAM,
        },
        logger::Logger,
    },
    dex::pump_fun::{INITIAL_VIRTUAL_SOL_RESERVES, INITIAL_VIRTUAL_TOKEN_RESERVES},
    engine::{monitor::BondingCurveInfo, swap::SwapDirection},
};

fn max_amount_with_slippage(input_amount: u64, slippage_bps: u64) -> u64 {
    input_amount
        .checked_mul(slippage_bps.checked_add(TEN_THOUSAND).unwrap())
        .unwrap()
        .checked_div(TEN_THOUSAND)
        .unwrap()
}

pub fn sol_token_quote(
    amount: u64,
    virtual_sol_reserves: u64,
    virtual_token_reserves: u64,
    is_buy: bool,
) -> u64 {
    if is_buy {
        (virtual_token_reserves as f64 / (amount as f64 + virtual_sol_reserves as f64)
            * (amount as f64)) as u64
    } else {
        (virtual_token_reserves as f64 / (amount as f64 + virtual_sol_reserves as f64 - 1_f64)
            * (amount as f64 + 1_f64)) as u64
    }
}

pub async fn build_pump_swap_ixn_by_cpi(
    swap_config: SwapConfig,
    mint_address: String,
    wallet: Arc<Keypair>,
    bonding_curve_info: Option<BondingCurveInfo>,
    logger: &Logger,
) -> Vec<Instruction> {
    // let start_time = Instant::now();

    let mint_address = Pubkey::from_str(&mint_address).unwrap();
    let payer_ata = get_associated_token_address(&wallet.pubkey(), &mint_address);
    let (pumpfun_global_acc, _bump_seed_pump_global) =
        Pubkey::find_program_address(&[PUMPFUN_GLOBAL], &PUMPFUN_PROGRAM_ADDRESS);
    let (pumpfun_bonding_curve, _bump_seed_bonding_curve) = Pubkey::find_program_address(
        &[BONDING_CURVE_SEED, &mint_address.to_bytes()],
        &PUMPFUN_PROGRAM_ADDRESS,
    );
    let pumpfun_bonding_curve_ata =
        get_associated_token_address(&pumpfun_bonding_curve, &mint_address);
    let (virtual_sol_reserves, virtual_token_reserves) =
        if let Some(bonding_curve_info) = bonding_curve_info {
            (
                bonding_curve_info.new_virtual_sol_reserve as u128,
                bonding_curve_info.new_virtual_token_reserve as u128,
            )
        } else {
            (
                INITIAL_VIRTUAL_SOL_RESERVES as u128,
                INITIAL_VIRTUAL_TOKEN_RESERVES as u128,
            )
        };
    let amount_in = ui_amount_to_amount(swap_config.amount_in, spl_token::native_mint::DECIMALS);

    if swap_config.swap_direction == SwapDirection::Buy {
        let discriminator = vec![102, 6, 61, 18, 1, 218, 235, 234];
        let mut data = discriminator;

        let slippage_bps = swap_config.slippage * 100;
        let max_sol_cost = max_amount_with_slippage(amount_in, slippage_bps);
        let amount_result = u128::from(amount_in)
            .checked_mul(virtual_token_reserves)
            .expect("Failed to multiply amount_specified by virtual_token_reserves: overflow occurred.")
            .checked_div(virtual_sol_reserves)
            .expect("Failed to divide the result by virtual_sol_reserves: division by zero or overflow occurred.");
        let token_amount = amount_result as u64;

        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&max_sol_cost.to_le_bytes());

        // Prepare the accounts vector
        let accounts = vec![
            AccountMeta::new_readonly(pumpfun_global_acc, false),
            AccountMeta::new(PUMPFUN_FEE_ACC, false),
            AccountMeta::new_readonly(mint_address, false),
            AccountMeta::new(pumpfun_bonding_curve, false),
            AccountMeta::new(pumpfun_bonding_curve_ata, false),
            AccountMeta::new(payer_ata, false),
            AccountMeta::new(wallet.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(SYSVAR_RENT_PUBKEY, false),
            AccountMeta::new_readonly(PUMPFUN_EVENT_AUTH, false),
            AccountMeta::new_readonly(PUMPFUN_PROGRAM_ADDRESS, false),
        ];
        let mut ixns = Vec::new();
        ixns.push(Instruction {
            program_id: PUMPFUN_PROGRAM_ADDRESS,
            accounts,
            data,
        });

        let create_wrapsol_ixn = create_associated_token_account(
            &wallet.pubkey(),
            &wallet.pubkey(),
            &mint_address,
            &spl_token::ID,
        );

        ixns.insert(0, create_wrapsol_ixn);
        // logger.log(
        //     format!("[IXN-ELLAPSED(BUY)]: {:?}", start_time.elapsed())
        //         .yellow()
        //         .to_string(),
        // );
        ixns
    } else {
        let discriminator = vec![51, 230, 133, 164, 1, 127, 131, 173]; // "sell" instruction discriminator
        let mut data = discriminator;
        let token_amount = amount_in;
        let min_sol_output = 0_u64;
        data.extend_from_slice(&token_amount.to_le_bytes());
        data.extend_from_slice(&min_sol_output.to_le_bytes());

        // Define the required accounts for CPI
        let accounts = vec![
            AccountMeta::new_readonly(pumpfun_global_acc, false),
            AccountMeta::new(PUMPFUN_FEE_ACC, false),
            AccountMeta::new_readonly(mint_address, false),
            AccountMeta::new(pumpfun_bonding_curve, false),
            AccountMeta::new(pumpfun_bonding_curve_ata, false),
            AccountMeta::new(payer_ata, false),
            AccountMeta::new(wallet.pubkey(), true),
            AccountMeta::new_readonly(SYSTEM_PROGRAM, false),
            AccountMeta::new_readonly(ASSOCIATED_TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(TOKEN_PROGRAM, false),
            AccountMeta::new_readonly(PUMPFUN_EVENT_AUTH, false),
            AccountMeta::new_readonly(PUMPFUN_PROGRAM_ADDRESS, false),
        ];

        // Create and return the instruction
        let mut ixns = Vec::new();
        ixns.push(Instruction {
            program_id: PUMPFUN_PROGRAM_ADDRESS,
            accounts,
            data,
        });
        ixns
    }
}
