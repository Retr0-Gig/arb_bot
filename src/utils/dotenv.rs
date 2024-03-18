use ethers::{prelude::*, providers::{Ipc, Provider}};
use std::str::FromStr;

// Construct the searcher wallet
pub fn get_searcher_wallet() -> LocalWallet {
    let searcher_private_key = std::env::var("SEARCHER_PRIVATE_KEY")
        .expect("Required environment variable \"SEARCHER_PRIVATE_KEY\" not set");
    searcher_private_key
        .parse::<LocalWallet>()
        .expect("Failed to parse private key")
}


/// Construct the bundle signer
/// This is your flashbots searcher identity
pub fn get_bundle_signer() -> LocalWallet {
    let private_key = std::env::var("FLASHBOTS_AUTH_KEY")
        .expect("Required environment variable \"FLASHBOTS_AUTH_KEY\" not set");
    private_key
        .parse::<LocalWallet>()
        .expect("Failed to parse flashbots signer")
}

/// Returns the configured Sandwich Contract Address
pub fn get_multicall_contract_address() -> Address {
    let addr = std::env::var("SANDWICH_CONTRACT")
        .expect("Required environment variable \"SANDWICH_CONTRACT\" not set");
    Address::from_str(&addr).expect("Failed to parse \"SANDWICH_CONTRACT\"")
}

/// Read environment variables
pub fn read_env_vars() -> Vec<(String, String)> {
    let mut env_vars = Vec::new();
    let keys = vec![
        "RPC_URL_WSS",
        "IPC_PATH",
        "SEARCHER_PRIVATE_KEY",
        "FLASHBOTS_AUTH_KEY",
        "SANDWICH_CONTRACT",
    ];
    for key in keys {
        let value = dotenv::var(key).expect(&format!(
            "Required environment variable \"{}\" not set",
            key
        ));
        env_vars.push((key.to_string(), value));
    }
    env_vars
}


/// Return a new ws provider
pub async fn get_ws_provider() -> Provider<Ws> {
    let url =
        dotenv::var("RPC_URL_WSS").expect("Required environment variable \"RPC_URL_WSS\" not set");
        Provider::<Ws>::connect(&url)
        .await
        .expect("RPC Connection Error")
}

//return a new ipc provider
pub async fn get_ipc_provider() -> Provider<Ipc> 
{

    let  path = 
        dotenv::var("IPC_PATH").expect("Required enviroment variable \"IPC_PATH\" not set");
    
         Provider::<Ipc>::connect_ipc(&path)
        .await
        .expect("Ipc Connection Error")
}



