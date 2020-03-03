use reqwest::header;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Duration;

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

pub fn balances<'a>(public_addr: &str, api_key: &'a str) -> Balances<'a> {
    let coins: Vec<Balance> = Vec::new();
    let b = Balance {
        symbol: "a",
        amount: 0.1,
    };
    Balances { coins: vec![b] }
}

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
    let status = resp.status();
    let balance_response = resp.json::<BalanceResponse>().unwrap();
    if balance_response.status == "1" {
        balance_response.result.parse::<f64>().unwrap()
    } else {
        println!("{:?}", balance_response);
        0.0
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
