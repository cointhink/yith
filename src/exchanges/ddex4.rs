use crate::config;
use crate::error;
use crate::eth;
use crate::exchange;
use crate::types;
use reqwest::header;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
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

pub struct Ddex4 {}

impl exchange::Api for Ddex4 {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeApi,
        market: &types::Market,
        offer: &types::Offer,
        proxy: &str,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        println!(
            "ddex4(hydro) build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );
        let mut market_id = make_market_id(market.swapped, &market.base, &market.quote);
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

        let mut token = String::from("");
        let fixedtime = format!(
            "{}{}",
            "HYDRO-AUTHENTICATION@",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
        );
        build_token(&mut token, privkey, fixedtime.as_str());
        let ddex_auth_headername = "Hydro-Authentication";
        let mut headers = header::HeaderMap::new();
        headers.insert(
            ddex_auth_headername,
            header::HeaderValue::from_str(&token).unwrap(), //boom
        );
        let resp = client.post(&url).headers(headers).json(&sheet).send()?;
        let status = resp.status();
        println!("{:#?} {}", resp.status(), resp.url());
        let body = resp.json::<BuildResponse>().unwrap();
        println!("{:#?}", body);
        if status.is_success() {
            Ok(exchange::OrderSheet::Ddex4(sheet))
        } else {
            Err(Box::new(error::OrderError::new(&body.desc)))
        }
    }

    fn submit(&self, sheet: exchange::OrderSheet) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

pub fn build_auth_client(proxy_url: &str) -> reqwest::Result<reqwest::blocking::Client> {
    let mut headers = header::HeaderMap::new();
    let bldr = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers);
    let bldr = if proxy_url.len() > 0 {
        println!("PROXY {}", proxy_url);
        let proxy = reqwest::Proxy::all(proxy_url)?;
        bldr.proxy(proxy)
    } else {
        bldr
    };
    bldr.build()
}

fn build_token(token: &mut String, privkey: &str, msg: &str) {
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
}

pub fn order(os: OrderSheet) {
    println!("HYDRO order! {:#?}", os);
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
