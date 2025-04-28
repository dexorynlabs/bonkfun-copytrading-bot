
use solana_vntr_pumpswap_copytrader::{common::{config::Config, constants::RUN_MSG}, engine::monitor::pumpswap_trader};

#[tokio::main]
async fn main() {
    /* Initial Settings */
    let config = Config::new().await;
    let config = config.lock().await;

    /* Running Bot */
    let run_msg = RUN_MSG;
    println!("{}", run_msg);

    let _ = pumpswap_trader(
        config.yellowstone_grpc_http.clone(),
        config.yellowstone_grpc_token.clone(),
        config.app_state.clone(),
        config.swap_config.clone(),
        config.targetlist.clone(),
    )
    .await;

}
