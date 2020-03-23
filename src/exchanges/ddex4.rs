use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use reqwest::header;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LimitMarket {
    #[serde(rename = "limit")]
    Limit,
    #[serde(rename = "market")]
    Market,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSheet {
    #[serde(rename = "marketId")]
    market_id: String,
    wallet_type: String,
    side: BuySell,
    #[serde(rename = "orderType")]
    order_type: LimitMarket,
    price: String,
    amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    status: i64,
    desc: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    id: String,
    r#type: String,
    version: String,
    status: String,
    side: String,
    price: String,
    amount: String,
    created_at: i64,
}

impl Order {
    pub fn to_exchange_order(&self) -> exchange::Order {
        let side = match self.r#type.as_str() {
            "buy" => Ok(exchange::BuySell::Sell),
            "sell" => Ok(exchange::BuySell::Buy),
            _ => Err(()),
        }
        .unwrap();
        let state = match self.status.as_str() {
            "pending" => Ok(exchange::OrderState::Open),
            "partial filled" => Ok(exchange::OrderState::Open),
            "full filled" => Ok(exchange::OrderState::Filled),
            "canceled" => Ok(exchange::OrderState::Cancelled),
            _ => Err(()),
        }
        .unwrap();
        let date = chrono::NaiveDateTime::from_timestamp(self.created_at, 0);
        exchange::Order {
            id: self.id.clone(),
            side: side,
            state: state,
            market: "UNK".to_string(),
            base_qty: f64::from_str(self.amount.as_str()).unwrap(),
            quote: f64::from_str(self.price.as_str()).unwrap(),
            create_date: date.timestamp(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderData {
    orders: Vec<Order>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    status: i64,
    desc: String,
    data: Option<OrderData>,
}

pub struct Ddex4 {}

impl exchange::Api for Ddex4 {
    fn setup(&mut self) {}

    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &types::Market,
        offer: &types::Offer,
        proxy: Option<String>,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        println!(
            "ddex4(hydro) build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );
        let market_id = make_market_id(market.swapped, &market.base, &market.quote);
        let mut qty = offer.base_qty;
        let mut price = offer.quote;
        let mut askbid_align = askbid;
        let askbid_other = askbid.otherside();
        if market.swapped {
            askbid_align = &askbid_other;
            let (s_qty, s_price) = offer.swap();
            println!(
                "unswapped {:#?} {} {}-{} {}@{}",
                askbid_align, market.source.name, market.quote, market.base, s_qty, s_price
            );
            qty = s_qty;
            price = s_price;
        }
        let side = match askbid_align {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        };

        let sheet = OrderSheet {
            market_id: market_id,
            side: side,
            order_type: LimitMarket::Limit,
            wallet_type: "trading".to_string(),
            price: format!("{:.width$}", price, width = market.price_decimals as usize),
            amount: format!("{:.width$}", qty, width = market.quantity_decimals as usize),
        };

        let client = build_auth_client(proxy)?;

        let url = format!("{}{}", exchange.api_url.as_str(), "/orders/build");
        println!("Ddex4 order {}", url);
        println!("{:#?}", &sheet);

        let headers = auth_header(privkey);
        println!("{}", serde_json::to_string(&sheet).unwrap());
        let resp = client.post(&url).headers(headers).json(&sheet).send()?;
        let status = resp.status();
        println!("{:#?} {}", resp.status(), resp.url());
        let body = resp.json::<BuildResponse>().unwrap();
        if status.is_success() {
            if body.status > 0 {
                let order_error = exchange::OrderError {
                    msg: body.desc,
                    code: body.status as i32,
                };
                println!("ERR: {}", order_error);
                Err(Box::new(order_error))
            } else {
                Ok(exchange::OrderSheet::Ddex4(sheet))
            }
        } else {
            let order_error = exchange::OrderError {
                msg: body.desc,
                code: body.status as i32,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    fn submit(
        &self,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("HYDRO order! {:#?}", sheet);
        Ok(())
    }

    fn open_orders(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<exchange::Order> {
        let client = reqwest::blocking::Client::new();
        let url = format!("{}/orders?marketId=all", exchange.api_url.as_str());
        println!("{}", url);
        let headers = auth_header(private_key);
        let resp = client.get(url.as_str()).headers(headers).send().unwrap();
        println!("{:#?} {}", resp.status(), resp.url());
        //println!("{:#?}", resp.text());
        let order_resp = resp.json::<OrderResponse>().unwrap();
        if order_resp.status < 0 {
            println!("ddex3 order list error {}", order_resp.desc);
            vec![]
        } else {
            order_resp
                .data
                .unwrap()
                .orders
                .iter()
                .map(|native_order| native_order.to_exchange_order())
                .collect()
        }
    }
}

pub fn build_auth_client(proxy_url: Option<String>) -> reqwest::Result<reqwest::blocking::Client> {
    let headers = header::HeaderMap::new();
    let bldr = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers);
    let bldr = match proxy_url {
        Some(proxy_url) => {
            println!("PROXY {}", proxy_url);
            let proxy = reqwest::Proxy::all(&proxy_url)?;
            bldr.proxy(proxy)
        }
        None => bldr,
    };
    bldr.build()
}

fn auth_header(privkey: &str) -> header::HeaderMap {
    let msg = format!(
        "{}{}",
        "HYDRO-AUTHENTICATION@",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );

    let token = build_token(privkey, &msg);
    let ddex_auth_headername = "Hydro-Authentication";
    let mut headers = header::HeaderMap::new();
    headers.insert(
        ddex_auth_headername,
        header::HeaderValue::from_str(&token).unwrap(), //boom
    );
    headers
}

fn build_token(privkey: &str, msg: &str) -> String {
    let mut token = "".to_string();
    let secp = Secp256k1::new();
    let privbytes = &hex::decode(privkey).unwrap();
    let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let pubkey_bytes = public_key.serialize_uncompressed();
    let addr = eth::pubkey_to_addr(pubkey_bytes);
    let msg_hash = eth::ethsign_hash_msg(&msg.as_bytes().to_vec());
    let sig_bytes = eth::sign_bytes(&msg_hash, &secret_key);
    token.push_str(
        format!(
            "0x{}#{}#0x{}",
            hex::encode(addr),
            msg,
            hex::encode(&sig_bytes[..])
        )
        .as_str(),
    );
    token
}

pub fn make_market_id(swapped: bool, base: &types::Ticker, quote: &types::Ticker) -> String {
    match swapped {
        true => format!("{}-{}", quote.symbol, base.symbol),
        false => format!("{}-{}", base.symbol, quote.symbol),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static privkey: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
    static good_addr: &str = "ed6d484f5c289ec8c6b6f934ef6419230169f534";
    static msg_v4: &str = "HYDRO-AUTHENTICATION@1566380397473";
    static msg_v3: &str = "HYDRO-AUTHENTICATION@1524088776656";
    static good_sig_v4: &str = "2a10e17a0375a6728947ae4a4ad0fe88e7cc8dd929774be0e33d7e1988f1985f13cf66267134ec4777878b6239e7004b9d2defb03ede94352a20acf0a20a50dc1b";
    static good_sig_v3: &str = "603efd7241bfb6c61f4330facee0f7027d98e030ef241ad03a372638c317859a50620dacee177b771ce05812770a637c4c7395da0042c94250f86fb52472f93500";

    #[test]
    fn test_build_token() {
        let mut token = String::from("");
        build_token(&mut token, privkey, msg_v4);
        let good_token = format!("0x{}#{}#0x{}", good_addr, msg_v4, good_sig_v4);
        assert_eq!(token, good_token);
    }
}
