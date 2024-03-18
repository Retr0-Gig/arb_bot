use ethers::types::I256;
use serde::Deserialize;
use std::cell::RefCell;
use std::str::FromStr;
use tokio::sync::MutexGuard;

// use crate::contract_modules::uniswap_v2::swap_math::get_amount_out;
use crate::constants::WETH;
use crate::contract_modules::uniswap_v2::types::UniV2Pool;
use crate::contract_modules::uniswap_v2::bindings::uni_v2_pair;
use crate::state::State;
use crate::utils::dotenv::get_multicall_contract_address;
use ethers::types::{Address, U256, Bytes};
use std::cmp::Ordering;
use ethers::abi::{Token, encode};


#[derive(Debug, Clone, Deserialize)]
pub struct NetPositiveCycle {
    pub profit: I256,
    pub optimal_in: U256,
    pub swap_amounts: Vec<(U256, bool)>,
    pub cycle_addresses: Vec<Address>,
}

impl Ord for NetPositiveCycle {
    fn cmp(&self, other: &Self) -> Ordering {
        other.profit.cmp(&self.profit)
    }
}

impl PartialOrd for NetPositiveCycle {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for NetPositiveCycle {}

// Ordering based on profit
impl PartialEq for NetPositiveCycle {
    fn eq(&self, other: &Self) -> bool {
        self.profit == other.profit
    }
}

impl NetPositiveCycle
{
    pub fn encode_data(&self) -> Bytes
    {

        let weth = Token::Address(Address::from_str(WETH).unwrap());
        let optimal = Token::Uint(self.optimal_in);
        let mut flashswap_calldata = Vec::<Token>::new();
        let mut pools = Vec::<Token>::new();
        let empty_bytes: Vec<u8> = Vec::new();

        flashswap_calldata.push(Token::Bytes(transfer_calldata(self.cycle_addresses[1], self.swap_amounts[0].0.clone()).to_vec()));
        pools.push(Token::Address(self.cycle_addresses[1]));

        for (idx, (amount, flag)) in self.swap_amounts.iter().skip(1).enumerate()
        {
        
         let next = if  idx > self.swap_amounts.len()  {
              self.cycle_addresses[idx+1]
            } else { get_multicall_contract_address() }; 


           let data = match flag 
                {
                    true => {

                        swap_calldata(amount.clone(), U256::zero(), next,empty_bytes.clone())

                        
                    },
                    false => {

                        swap_calldata(U256::zero(), amount.clone(), next,empty_bytes.clone())
                    }
                };

            flashswap_calldata.push(Token::Bytes(data.to_vec()));          

        }

        for pool in self.cycle_addresses.iter().skip(1)
        {
            pools.push(Token::Address(pool.clone()));
        }


        let tokens = vec![
            weth,
            optimal,
            Token::Array(pools),
            Token::Array(flashswap_calldata),
        ];

        let swap_data = match self.swap_amounts[0].1
        {
            true => {
                swap_calldata( self.optimal_in, U256::zero(), get_multicall_contract_address(), Bytes::from(encode(&tokens)).to_vec())

                
            },
            false => {
                swap_calldata(U256::zero(), self.optimal_in, get_multicall_contract_address(),Bytes::from(encode(&tokens)).to_vec())
            }
        };


        swap_data        

    }
}

pub fn find_optimal_cycles(
    state: &MutexGuard<State>,
    affected_pairs: Option<Vec<Address>>,
) -> Vec<NetPositiveCycle> {
    let mut pointers: Vec<&Vec<crate::state::IndexedPair>> = Vec::new();

    match affected_pairs {
        Some(affected_pairs) => {
            affected_pairs.iter().for_each(|pair_address| {
                if let Some(cycle) = state.cycles_mapping.get(pair_address) {
                    pointers.extend(cycle.iter());
                }                
            });   
        }
        None => {
            for (_, cycles) in &state.cycles_mapping {
                pointers.extend(cycles.iter());
            }
        }
    }

    let mut net_profit_cycles = Vec::new();

    let weth = Address::from_str(WETH).unwrap();
    for cycle in pointers {
        let pairs = cycle
            .iter()
            .filter_map(|pair| state.pairs_mapping.get(&pair.address))
            .collect::<Vec<&RefCell<UniV2Pool>>>();

        let pairs_clone = pairs.clone();
        let profit_function =
            move |amount_in: U256| -> I256 { get_profit(weth, amount_in, &pairs_clone) };

        let optimal = maximize_profit(
            U256::one(),
            U256::from_dec_str("10000000000000000000000").unwrap(),
            U256::from_dec_str("10").unwrap(),
            profit_function,
        );

        let (profit, swap_amounts) = get_profit_with_amount(weth, optimal, &pairs);

        let mut cycle_internal = Vec::new();
        for pair in pairs {
            cycle_internal.push(pair.borrow().address);
        }

        if profit > I256::one() {
            let net_positive_cycle = NetPositiveCycle {
                profit,
                optimal_in: optimal,
                cycle_addresses: cycle_internal,
                swap_amounts,
            };

            net_profit_cycles.push(net_positive_cycle);
        }
    }

    net_profit_cycles.sort();
    net_profit_cycles.into_iter().take(5).collect()
}

// find optimal input before uni fees eats away our profits
// Quadratic search
fn maximize_profit(
    mut domain_min: U256,
    mut domain_max: U256,
    lowest_delta: U256,
    f: impl Fn(U256) -> I256,
) -> U256 {
    loop {
        if domain_max > domain_min {
            if (domain_max - domain_min) > lowest_delta {
                let mid = (domain_min + domain_max) / 2;

                let lower_mid = (mid + domain_min) / 2;
                let upper_mid = (mid + domain_max) / 2;

                let f_output_lower = f(lower_mid);
                let f_output_upper = f(upper_mid);

                if f_output_lower > f_output_upper {
                    domain_max = mid;
                } else {
                    domain_min = mid;
                }
            } else {
                break;
            }
        } else {
            break;
        }
    }

    (domain_max + domain_min) / 2
}

/// Calculates profit given (state updated) pairs
pub fn get_profit(token_in: Address, amount_in: U256, pairs: &Vec<&RefCell<UniV2Pool>>) -> I256 {
    let mut amount_out: U256 = amount_in;
    let mut token_in = token_in;
    for pair in pairs {
        let pair = pair.borrow();
        let fees;
        let (reserve0, reserve1) = if pair.token0 == token_in {
            fees = pair.fees1;
            (pair.reserve0, pair.reserve1)
        } else {
            fees = pair.fees0;
            (pair.reserve1, pair.reserve0)
        };
        amount_out = get_amount_out(amount_out, reserve0, reserve1, fees, pair.router_fee);
        token_in = if pair.token0 == token_in {
            pair.token1
        } else {
            pair.token0
        };
    }

    I256::from_raw(amount_out) - I256::from_raw(amount_in)
}

pub fn get_profit_with_amount(
    token_in: Address,
    amount_in: U256,
    pairs: &Vec<&RefCell<UniV2Pool>>,
) -> (I256, Vec<(U256, bool)>) {
    let mut amount_out: U256 = amount_in;
    let mut token_in = token_in;
    let mut amounts = Vec::with_capacity(pairs.len() + 1);
    let first_value = &token_in == &pairs[0].borrow().token0;
    amounts.push((amount_in, first_value));

    for pair in pairs {
        let pair = pair.borrow();
        let fees;
        let (reserve0, reserve1) = if pair.token0 == token_in {
            fees = pair.fees1;
            (pair.reserve0, pair.reserve1)
        } else {
            fees = pair.fees0;
            (pair.reserve1, pair.reserve0)
        };
        amount_out = get_amount_out(amount_out, reserve0, reserve1, fees, pair.router_fee);
        
        token_in = if pair.token0 == token_in {
            amounts.push((amount_out, true));
            pair.token1
            
        } else {
            amounts.push((amount_out, false));
            pair.token0            
        };
    }

    (
        I256::from_raw(amount_out) - I256::from_raw(amount_in),
        amounts,
    )
}

// We don't want overflow / underflow at runtime + need to be a bit fast
pub fn get_amount_out(
    a_in: U256,
    reserve_in: U256,
    reserve_out: U256,
    fees: U256,
    router_fee: U256,
) -> U256 {
    if a_in == U256::zero() {
        return U256::zero();
    }
    let a_in_with_fee = a_in.saturating_mul(router_fee);
    let a_out = a_in_with_fee.saturating_mul(reserve_out)
        / U256::from(10000)
            .saturating_mul(reserve_in)
            .saturating_add(a_in_with_fee);

    a_out - a_out.saturating_mul(fees) / U256::from(10000)
}


pub fn swap_calldata(
    amount_0_out: U256,
    amount_1_out: U256,
    to: Address,
    calldata: Vec<u8>,
) -> Bytes {
    let input_tokens = vec![
        Token::Uint(amount_0_out),
        Token::Uint(amount_1_out),
        Token::Address(to),
        Token::Bytes(calldata),
    ];

    uni_v2_pair::IUNISWAPV2PAIR_ABI
        .function("swap")
        .unwrap()
        .encode_input(&input_tokens)
        .expect("Could not encode swap calldata").into()
}

pub fn transfer_calldata(
    recipient: Address,
    amount: U256,
    ) -> Bytes {

        let input_tokens = vec![
            Token::Address(recipient),
            Token::Uint(amount),
        ];

        uni_v2_pair::IERC20_ABI
            .function("transfer")
            .unwrap()
            .encode_input(&input_tokens)
            .expect("Could not encode transfer calldata").into()

}
