use crate::exchanges;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InternalTransaction {
    pub block_number: String,
    pub from: String,
    pub value: String,
    pub gas_used: String,
    pub is_error: String,
    pub err_code: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Erc20Transaction {
    pub block_number: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas_used: String,
    pub token_name: String,
    pub token_symbol: String,
}

#[derive(Debug)]
pub struct Etherscan {
    pub tokens: TokenList,
    pub api_key: String,
}

impl Etherscan {
    pub fn new(api_key: &str) -> Etherscan {
        let tokens = read_tokens("./notes/etherscan-tokens.json");
        Etherscan {
            tokens: tokens,
            api_key: api_key.to_string(),
        }
    }
}

type TokenList = exchanges::switcheo::TokenList;

pub fn read_tokens(filename: &str) -> TokenList {
    let yaml = fs::read_to_string(filename).unwrap();
    let tokens = serde_yaml::from_str(&yaml).unwrap();
    TokenList { tokens: tokens }
}

pub struct Balances<'a> {
    coins: Vec<Balance<'a>>,
}

impl<'a> fmt::Display for Balances<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for balance in &self.coins {
            write!(f, "{},", balance)?
        }
        write!(f, "")
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T> {
    status: String,
    result: Vec<T>,
}

pub struct Balance<'a> {
    symbol: &'a str,
    amount: f64,
}

impl<'a> fmt::Display for Balance<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.symbol, self.amount)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    status: String,
    message: String,
    result: String,
}

static ETHERSCAN_API_URL: &'static str = "https://api.etherscan.io/api";

pub fn balance<'a>(public_addr: &str, contract: &str, api_key: &'a str) -> f64 {
    let client = build_client(api_key).unwrap();
    let url_params = match contract {
        "0x0000000000000000000000000000000000000000" => {
            format!("module=account&action=balance&address=0x{}", public_addr)
        }
        _ => format!(
            "module=account&action=tokenbalance&contractaddress={}&address=0x{}",
            contract, public_addr
        ),
    };
    let url = format!(
        "{}?{}&tag=latest&apikey={}",
        ETHERSCAN_API_URL, url_params, api_key
    );
    let resp = client.get(&url).send().unwrap();
    if resp.status().is_success() {
        let balance_response = resp.json::<BalanceResponse>().unwrap();
        if balance_response.status == "1" {
            balance_response.result.parse::<f64>().unwrap()
        } else {
            println!("{:?}", balance_response);
            0.0
        }
    } else {
        0.0 // err handling
    }
}

pub fn last_token_transaction(
    public_addr: &str,
    start_block: u64,
    token: &str,
    api_key: &str,
) -> Result<Erc20Transaction, String> {
    let client = build_client(api_key).unwrap();
    let url = format!(
        "{}?module=account&action=tokentx&address=0x{}&startblock={}&sort=desc&apikey={}",
        ETHERSCAN_API_URL, public_addr, start_block, api_key
    );
    let resp = client.get(&url).send().unwrap();
    println!(
        "etherscan tokentx looking for {} since block {} => {}",
        token,
        start_block,
        resp.status(),
    );
    if resp.status().is_success() {
        let response = resp.json::<ApiResponse<Erc20Transaction>>().unwrap();
        let good: Vec<&Erc20Transaction> = response
            .result
            .iter()
            .filter(|t| t.token_symbol == token)
            .collect();
        if good.len() > 0 {
            Ok(good[0].clone())
        } else {
            Err("no internal transactions".to_string())
        }
    } else {
        Err("etherscan bad".to_string())
    }
}

pub fn last_internal_transaction_from(
    public_addr: &str,
    start_block: u64,
    from_addr: &str,
    api_key: &str,
) -> Result<InternalTransaction, String> {
    let client = build_client(api_key).unwrap();
    let url = format!(
        "{}?module=account&action=txlistinternal&address=0x{}&startblock={}&sort=desc&apikey={}",
        ETHERSCAN_API_URL, public_addr, start_block, api_key
    );
    let resp = client.get(&url).send().unwrap();
    println!("{} {}", url, resp.status());
    if resp.status().is_success() {
        let response = resp.json::<ApiResponse<InternalTransaction>>().unwrap();
        let good: Vec<&InternalTransaction> = response
            .result
            .iter()
            .filter(|t| t.from == from_addr && t.is_error == "0")
            .collect();
        if good.len() > 0 {
            Ok(good[0].clone())
        } else {
            Err(format!("no internal transactions from {}", from_addr))
        }
    } else {
        Err("etherscan bad".to_string())
    }
}

pub fn build_client(api_key: &str) -> reqwest::Result<reqwest::blocking::Client> {
    let mut headers = header::HeaderMap::new();
    let token = format!("Bearer {}", api_key);
    headers.insert(
        "authorization",
        header::HeaderValue::from_str(&token).unwrap(), //boom
    );
    let bldr = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers);
    bldr.build()
}
