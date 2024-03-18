use std::sync::Arc;
use dashmap::DashMap;
use crate::utils;
use crate::constants;

use ethers::prelude::*;
use eyre::Result;
use tokio::sync::RwLock;



#[derive(Clone, Debug)]
/// Holds the state of the bot
pub struct BotState {
   pub multicall_balance: DashMap<Address, Arc<RwLock<U256>>>,
}

impl BotState {
    // Create a new instance of the bot state
    //
    // Arguments:
    // * `sandwich_inception_block`: block number sandwich was deployed
    // * `client`: websocket provider to use for fetching data
    //
    // Returns:
    // Ok(BotState) if successful
    // Err(eyre::Error) if failed to create instance
    pub async fn new(client: &Arc<Provider<Ws>>) -> Result<Self> {
        
        let origin_tokens = constants::get_token_address();
        let multicall_balance = DashMap::new();

        for token in origin_tokens{        

            let token_contract =
                utils::get_erc20_contract(&token, &client);

            let token_balance = token_contract
                .balance_of(utils::dotenv::get_multicall_contract_address())
                .call()
                .await?;
            
            
           

            let token_balance = Arc::new(RwLock::new(token_balance));

            multicall_balance.insert(token_contract.address(), token_balance);

        }




        Ok(BotState {
            multicall_balance,
        })
    }

    

    // Update the WETH balance of the contract
    //
    // Arguments:
    // * `&self`: reference to `BotState` instance
    //
    // Returns: nothing
    pub async fn update_multicall_balance(&self) {

        for (token, balance) in self.multicall_balance.to_owned().into_iter() {

            
            let client = utils::create_websocket_client()
                                                  .await.unwrap();

            let token_contract =
            utils::get_erc20_contract(&token, &client);

            let token_balance = token_contract
                .balance_of(utils::dotenv::get_multicall_contract_address())
                .call()
                .await.unwrap();
                
            let mut lock = balance.write().await;

            *lock = token_balance;
        }
        

    }

}
