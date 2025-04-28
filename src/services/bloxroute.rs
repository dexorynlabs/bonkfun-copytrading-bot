use anyhow::{anyhow, Result};
use rand::seq::IteratorRandom;
use rand::thread_rng;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::{signature::Signature, signer::Signer, transaction::Transaction};
use solana_trader_client_rust::{common::signing::SubmitParams, provider::grpc::GrpcClient};
use solana_trader_proto::api::TransactionMessage;
use std::str::FromStr;

use base64::engine::general_purpose;
use base64::Engine;

pub fn get_tip_account() -> Result<Pubkey> {
    let accounts = [
        "HWEoBxYs7ssKuudEjzjmpfJVX7Dvi7wescFsVx2L5yoY".to_string(),
        "95cfoy472fcQHaw4tPGBTKpn6ZQnfEPfBgDQx6gcRmRg".to_string(),
    ];
    let mut rng = thread_rng();
    let tip_account = match accounts.iter().choose(&mut rng) {
        Some(acc) => Ok(Pubkey::from_str(acc).inspect_err(|err| {
            println!("jito: failed to parse Pubkey: {:?}", err);
        })?),
        None => Err(anyhow!("jito: no tip accounts available")),
    };

    let tip_account = tip_account?;
    Ok(tip_account)
}

pub async fn get_tip_value() -> Result<f64> {
    // If TIP_VALUE is set, use it
    if let Ok(tip_value) = std::env::var("BLOXROUTE_TIP_VALUE") {
        match f64::from_str(&tip_value) {
            Ok(value) => Ok(value),
            Err(_) => {
                println!(
                    "Invalid BLOXROUTE_TIP_VALUE in environment variable: '{}'. Falling back to percentile calculation.",
                    tip_value
                );
                Err(anyhow!("Invalid TIP_VALUE in environment variable"))
            }
        }
    } else {
        Err(anyhow!("BLOXROUTE_TIP_VALUE environment variable not set"))
    }
}

pub struct BloxRouteClient {
    client: GrpcClient,
    submit_opts: SubmitParams,
}

impl BloxRouteClient {
    pub async fn new() -> Result<Self> {
        let client = GrpcClient::new(None).await?;
        Ok(Self {
            client,
            submit_opts: SubmitParams::default(),
        })
    }
    pub async fn send_transaction(&mut self, transaction: &Transaction) -> Result<Vec<String>> {
        let mut txn = transaction.clone();
        let message_data = txn.message.serialize();
        txn.signatures = vec![Signature::default()];
        let keypair = self.client.get_keypair()?;
        txn.signatures[0] = keypair.sign_message(&message_data);

        let serialized_tx = bincode::serialize(&txn)?;
        let messages = vec![TransactionMessage {
            content: general_purpose::STANDARD.encode(serialized_tx),
            is_cleanup: false,
        }];

        self.submit_opts.use_staked_rpcs = true;
        let sig = self
            .client
            .sign_and_submit(messages, self.submit_opts.clone(), false)
            .await?;
        Ok(sig)
    }
}
