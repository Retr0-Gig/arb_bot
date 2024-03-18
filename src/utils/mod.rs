pub mod dotenv;

use ethers::{prelude::*, types::transaction::eip2718::TypedTransaction};
use std::{
    collections::{btree_map::Entry, BTreeMap},
    sync::Arc,
};

use crate::contract_modules::uniswap_v2::bindings::uni_v2_pair::{
   i_erc_20::IErc20, i_uniswap_v2_pair::IUniswapV2Pair
};

pub async fn get_state_diffs(
    client: &Arc<Provider<Ws>>,
    meats: &Vec<Transaction>,
    block_num: BlockNumber,
) -> Option<BTreeMap<Address, AccountDiff>> {
    // add statediff trace to each transaction
    let req = meats
        .iter()
        .map(|tx| (tx, vec![TraceType::StateDiff]))
        .collect();

    let block_traces = match client.trace_call_many(req, Some(block_num)).await {
        Ok(x) => x,
        Err(_) => {
            return None;
        }
    };

    let mut merged_state_diffs = BTreeMap::new();

    block_traces
        .into_iter()
        .flat_map(|bt| bt.state_diff.map(|sd| sd.0.into_iter()))
        .flatten()
        .for_each(|(address, account_diff)| {
            match merged_state_diffs.entry(address) {
                Entry::Vacant(entry) => {
                    entry.insert(account_diff);
                }
                Entry::Occupied(_) => {
                    // Do nothing if the key already exists
                    // we only care abt the starting state
                }
            }
        });

    Some(merged_state_diffs)
}

pub async fn get_logs(
    client: &Arc<Provider<Ws>>,
    tx: &Transaction,
    block_num: BlockNumber,
) -> Option<Vec<CallLogFrame>> {
    // add statediff trace to each transaction

    let mut trace_ops = GethDebugTracingCallOptions::default();
    let mut call_config = CallConfig::default();
    call_config.with_log = Some(true);

    trace_ops.tracing_options.tracer = Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer));
    trace_ops.tracing_options.tracer_config = Some(
        GethDebugTracerConfig::BuiltInTracer(
            GethDebugBuiltInTracerConfig::CallTracer(
                call_config
            )
        )
    );
    let block_num = Some(BlockId::Number(block_num));

    let call_frame = match client.debug_trace_call(tx, block_num, trace_ops).await {
        Ok(d) => {
            match d {
                GethTrace::Known(d) => {
                    match d {
                        GethTraceFrame::CallTracer(d) => d,
                        _ => return None
                    }
                }
                _ => return None
            }
        },
        Err(_) => {
            return None
        }
    }; 

    let mut logs = Vec::new();
    extract_logs(&call_frame, &mut logs);
    
    
    Some(logs)
}

fn extract_logs(call_frame: &CallFrame, logs: &mut Vec<CallLogFrame>) {
    if let Some(ref logs_vec) = call_frame.logs {
        logs.extend(logs_vec.iter().cloned());
    }

    if let Some(ref calls_vec) = call_frame.calls {
        for call in calls_vec {
            extract_logs(call, logs);
        }
    }
}

// Sign eip1559 transactions
pub async fn sign_eip1559(
    tx: Eip1559TransactionRequest,
    signer_wallet: &LocalWallet,
) -> Result<Bytes, WalletError> {
    let tx_typed = TypedTransaction::Eip1559(tx);
    let signed_frontrun_tx_sig = match signer_wallet.sign_transaction(&tx_typed).await {
        Ok(s) => s,
        Err(e) => return Err(e),
    };

    Ok(tx_typed.rlp_signed(&signed_frontrun_tx_sig))
}

/// Create Websocket Client
pub async fn create_websocket_client() -> eyre::Result<Arc<Provider<Ws>>> {
    let client = dotenv::get_ws_provider().await;
    Ok(Arc::new(client))
}

pub async fn get_nonce(
    client: &Arc<Provider<Ws>>,
    address: Address,
) -> Result<U256, ProviderError> {
    client.get_transaction_count(address, None).await
}

// Create erc20 contract that we can interact with
pub fn get_erc20_contract<M: Middleware>(
    erc20_address: &Address,
    client: &Arc<M>,
) -> IErc20<M> {
    IErc20::new(*erc20_address, client.clone())
}

/// Create v2 pair contract that we can interact with
pub fn get_pair_v2_contract<M: Middleware>(
    pair_address: &Address,
    client: &Arc<M>,
) -> IUniswapV2Pair<M> {
    IUniswapV2Pair::new(*pair_address, client.clone())
}
