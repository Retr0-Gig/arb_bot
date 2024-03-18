use std::sync::Arc;
use ethers::prelude::{rand::Rng, BlockId, *};
use crate::states::block_state::BlockInfo;
use crate::backrunner::BackRunner;
use crate::calc::NetPositiveCycle;
use crate::bundle_errors::SendBundleError;
use crate::utils;
use crate::relay;


// Construct and send bundle based on recipe
//
// Arguments:
// * `&recipe`: information on how to construct sandwich bundle
// * `target_block`: holds basefee and timestamp of target block
// * `sandwich_maker`: holds signer, bot address for constructing frontslice and backslice
//
// Returns:
// Ok(()): return nothing if sent succesful
// Err(SendBundleError): return error if send bundle fails
pub async fn send_bundle(
    backrun_tx: Transaction,
    recipes: Vec<Bytes>,
    net_positive: Vec<NetPositiveCycle>,
    backrunner: Arc<BackRunner>,
    target_block: BlockInfo,
    client: Arc<Provider<Ws>>
) -> Result<(), SendBundleError> {

   
    let nonce = {
        let read_lock = backrunner.nonce.read().await;
        (*read_lock).clone()
    };

    let backrun_tx_byte: Bytes =  backrun_tx.rlp();
    let mut raw_signed_txs = Vec::new();
    let mut count = U256::zero();

    for (idx, recipe) in recipes.clone().into_iter().enumerate() {

        let revenue: U256 = net_positive[idx].profit.into_raw();


        let arbitrage_request = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(backrunner.multicall_address)),
            from: Some(backrunner.searcher_wallet.address()),
            data: Some(recipe.clone()),
            chain_id: Some(U64::from(1)),
            ..Default::default()
        };

        let arb_tx = transaction::eip2718::TypedTransaction::Eip1559(arbitrage_request);
        let block = Some(BlockId::Number(BlockNumber::from(target_block.number-1)));

        let gas_used = match client.estimate_gas(&arb_tx, block).await {

            Ok(value) => { value },
            Err(_e)  => { return Err(SendBundleError::GasEstimateError())},
        };
       

        let max_fee = calculate_bribe_for_max_fee(gas_used, revenue, &target_block)?;

        let arbitrage_request = Eip1559TransactionRequest {
            to: Some(NameOrAddress::Address(backrunner.multicall_address)),
            from: Some(backrunner.searcher_wallet.address()),
            data: Some(recipe.clone()),
            chain_id: Some(U64::from(1)),
            max_priority_fee_per_gas: Some(max_fee),
            max_fee_per_gas: Some(max_fee),
            gas: Some((U256::from(gas_used) * 10) / 7),
            nonce: Some(nonce + count), // gasused = 70% gaslimit
           ..Default::default()
        };


        let arbitrage_tx =
            utils::sign_eip1559(arbitrage_request, &backrunner.searcher_wallet).await?;

        raw_signed_txs.push(arbitrage_tx);
        count += U256::from(1);
    }


    let nonce = (nonce + count).checked_sub(U256::from(1)).unwrap();
    

    let bundle = relay::construct_bundle(
        {
            let mut bundled_transactions: Vec<Bytes> = vec![backrun_tx_byte];
            for meat in raw_signed_txs {
                bundled_transactions.push(meat.clone());
            }
            bundled_transactions
        },
        target_block.number,
        target_block.timestamp.as_u64(),
    );


    // send bundle to all relay endpoints (concurrently)
    for relay in relay::get_all_relay_endpoints().await {
        let bundle = bundle.clone();
        let recipes = recipes.clone();
        let backrunner = backrunner.clone();

        tokio::spawn(async move {
            let pending_bundle = match relay.flashbots_client.inner().send_bundle(&bundle).await {
                Ok(pb) => pb,
                Err(_) => {
                    //log::error!("Failed to send bundle: {:?}", e);
                    return;
                }
            };

            log::info!(
                "{:?}",
                format!("Bundle sent to {}", relay.relay_name)
            );

            let bundle_hash = pending_bundle.bundle_hash;

            let is_bundle_included = match pending_bundle.await {
                Ok(_) => true,
                Err(ethers_flashbots::PendingBundleError::BundleNotIncluded) => false,
                Err(e) => {
                    log::error!(
                        "{:?} Bundle rejected due to error : {:?}",
                        recipes,
                        e
                    );
                    false
                }
            };

            log::info!("bundle hash: {:?} ", bundle_hash);

            match is_bundle_included {
                true => {backrunner.nonce.write().await.checked_add(nonce).unwrap();}
                false => {log::info!("bundle not included in block, nonce not increased")}
            }

            
        });
    }

    Ok(())
}



// calculates the optimal bribe for a given opportunity
//
// Arguments
// * `recipe`: information on sandwich bundle
// * `target_block`: information on target_block
//
// Returns:
// Ok(U256) -> The maximum fee for opportunity if calculated succesfully
// Err(SendBundleError) -> Error in bribe amount calculation
fn calculate_bribe_for_max_fee(
    gas_used: U256,
    revenue: U256,
    target_block: &BlockInfo,
) -> Result<U256, SendBundleError> {
    // frontrun txfee is fixed, exclude it from bribe calculations
    let revenue_minus_tx_fee = match 
        revenue
        .checked_sub(gas_used * target_block.base_fee)
    {
        Some(revenue) => revenue,
        None => return Err(SendBundleError::GasFeesNotCovered()),
    };



    // overpay to get dust onto sandwich contractIf
    // more info: https://twitter.com/libevm/status/1474870661373779969
    let bribe_amount = {
        let mut rng = rand::thread_rng();

        // enchanement: make bribe adaptive based on competitors    
        (revenue_minus_tx_fee * (700000000 + rng.gen_range(0..10000000)))
                    / 1000000000
    };

    // calculating bribe amount
    let max_fee: U256 = bribe_amount / gas_used;

    if max_fee < target_block.base_fee {
        return Err(SendBundleError::MaxFeeLessThanNextBaseFee());
    }

    let effective_miner_tip = max_fee.checked_sub(target_block.base_fee);

    if effective_miner_tip.is_none() {
        return Err(SendBundleError::NegativeMinerTip());
    }

    Ok(max_fee)
}
