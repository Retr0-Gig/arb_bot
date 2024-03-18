use std::sync::Arc;

use crate::utils;
use ethers::prelude::{k256::ecdsa::SigningKey, *};
use tokio::sync::RwLock;




#[derive(Debug, Clone)]
pub struct BackRunner {
    pub multicall_address: Address,
    pub searcher_wallet: Wallet<SigningKey>,
    pub nonce: Arc<RwLock<U256>>,
}

impl BackRunner {
    // Create a new `SandwichMaker` instance
    pub async fn new() -> Self {
        let multicall_address = utils::dotenv::get_multicall_contract_address();
        let searcher_wallet = utils::dotenv::get_searcher_wallet();

        let client = utils::create_websocket_client().await.unwrap();

        let nonce = if let Ok(n) = client
            .get_transaction_count(searcher_wallet.address(), None)
            .await
        {
            n
        } else {
            panic!("Failed to get searcher wallet nonce...");
        };

        let nonce = Arc::new(RwLock::new(nonce));

        Self {
            multicall_address,
            searcher_wallet,
            nonce,
        }
    }
}

/// Return the divisor used for encoding call value (weth amount)
pub fn get_weth_encode_divisor() -> U256 {
    U256::from(100000)
}
