use crate::config;
use crate::eth;
use crate::exchange;
use crate::exchanges::ddex::Ddex;
use crate::types;
use reqwest::header;
use secp256k1::{PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use std::fs;
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
pub struct MarketOrderSheet {
    market_id: String,
    side: BuySell,
    order_type: LimitMarket,
    price: String,
    amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    status: i64,
    desc: String,
    data: Option<BuildData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildData {
    order: OrderSheet,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSheet {
    id: String,
    market_id: String,
    side: BuySell,
    price: String,
    amount: String,
    maker_fee_rate: String,
    taker_fee_rate: String,
    as_maker_fee_rate: String,
    as_taker_fee_rate: String,
    maker_rebate_rate: String,
    gas_fee_amount: String,
    r#type: String,
}

/*    "order": {
      "id": "0x9b976813b83eb076a32f167a9dfcbc69a6df3f83bf524992313fe0601b27c9fd",
      "marketId": "BOMB-WETH",
      "side": "buy",
      "price": "0.0016510",
      "amount": "12",
      "json": {
        "trader": "0x9b827e7ee9f127a24eb5243e839007c417c8ac18",
        "relayer": "0x49497a4d914ae91d34ce80030fe620687bf333fd",
        "baseToken": "0x1c95b093d6c236d3ef7c796fe33f9cc6b8606714",
        "quoteToken": "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2",
        "baseTokenAmount": "12",
        "quoteTokenAmount": "19812000000000000",
        "gasTokenAmount": "950000000000000",
        "data": "0x02000004d6acf6ab0064012c000002778e201379eefc00000000000000000000"
      },
      "makerFeeRate": "0.00100",
      "takerFeeRate": "0.00300",
      "asMakerFeeRate": "0.00100",
      "asTakerFeeRate": "0.00300",
      "makerRebateRate": "0",
      "gasFeeAmount": "0.00095",
      "type": "limit"
*/

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderPlace {
    order_id: String,
    signature: String,
    method: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PairList {
    pairs: Vec<Pair>,
}

impl PairList {
    pub fn from_file(filename: &str) -> PairList {
        let file_ok = fs::read_to_string(filename);
        let yaml = file_ok.unwrap();
        let pairs = serde_yaml::from_str::<Vec<Pair>>(&yaml).unwrap();
        PairList { pairs: pairs }
    }

    pub fn get(&self, market: &str) -> Option<&Pair> {
        let mut result: Option<&Pair> = None;
        for pair in &self.pairs {
            if pair.id == market {
                result = Some(&pair);
                break
            };
        }
        result
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pair {
    id: String,
    price_decimals: i32,
    amount_decimals: i32,
}

pub struct Ddex3 {
    pairs: PairList,
    settings: config::ExchangeSettings,
}

impl Ddex3 {
    pub fn new(settings: config::ExchangeSettings) -> Ddex3 {
        let pairs = PairList::from_file("notes/ddex3-pairs.json");
        println!("ddex3 loaded {} pairs", pairs.pairs.len());
        Ddex3 {
            pairs: pairs,
            settings: settings,
        }
    }
}

impl Ddex for Ddex3 {}

impl exchange::Api for Ddex3 {
    fn setup(&mut self) {}

    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        println!(
            "ddex3(hydro) build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );
        let market_id = self.make_market_id(market);
        let qty = offer.base_qty;
        let price = offer.quote;
        let side = match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        };

        let pair = self.pairs.get(&market_id).unwrap();
        let sheet = MarketOrderSheet {
            market_id: market_id,
            side: side,
            order_type: LimitMarket::Limit,
            //wallet_type: "trading",
            price: format!("{:.width$}", price, width = pair.price_decimals as usize),
            amount: format!("{:.width$}", qty, width = pair.amount_decimals as usize),
        };

        let client = build_http_client(exchange)?;

        let url = format!("{}{}", exchange.api_url.as_str(), "/orders/build");
        println!("Ddex3 {}", url);

        let headers = auth_header(privkey);
        println!("{}", serde_json::to_string(&sheet).unwrap());
        let resp = client.post(&url).headers(headers).json(&sheet).send()?;
        let status = resp.status();
        println!("{:#?} {}", resp.status(), resp.url());
        let json = resp.text().unwrap();
        println!("{}", json);
        let body = serde_json::from_str::<BuildResponse>(&json).unwrap();
        if status.is_success() {
            if body.status > 0 {
                let err_msg = format!("{} {}", &body.status, &body.desc);
                let order_error = exchange::OrderError {
                    msg: body.desc,
                    code: body.status as i32,
                };
                println!("ERR: {}", order_error);
                Err(Box::new(order_error))
            } else {
                println!("{:?}", body.data);
                if let Some(order_build) = body.data {
                    Ok(exchange::OrderSheet::Ddex3(order_build.order))
                } else {
                    let order_error = exchange::OrderError {
                        msg: body.desc,
                        code: body.status as i32,
                    };
                    println!("ERR: {}", order_error);
                    Err(Box::new(order_error))
                }
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
        private_key: &str,
        exchange: &config::ExchangeSettings,
        sheet_opt: exchange::OrderSheet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let exchange::OrderSheet::Ddex3(sheet) = sheet_opt {
            println!("HYDRO order! {:#?}", sheet);
            let privbytes = &hex::decode(private_key).unwrap();
            let secret_key = SecretKey::from_slice(privbytes).unwrap();
            let signature = eth::ethsign_vrs(&sheet.id, &secret_key);
            let order_place = OrderPlace {
                order_id: sheet.id.clone(),
                signature: signature,
                method: 0,
            };
            println!("{:?}", order_place);
            let client = build_http_client(exchange)?;
            let url = format!("{}/orders/sync", exchange.api_url.as_str());
            println!("{}", url);
            let headers = auth_header(private_key);
            let resp = client
                .post(&url)
                .headers(headers)
                .json(&order_place)
                .send()?;
            let status = resp.status();
            let json = resp.text().unwrap();
            println!("{}", json);
            let response = serde_json::from_str::<BuildResponse>(&json).unwrap();
            println!("{:#?} {} {:?}", status, url, json);
            if response.status == 0 {
                Ok(())
            } else {
                let order_error = exchange::OrderError {
                    msg: response.desc,
                    code: response.status as i32,
                };
                println!("ERR: {}", order_error);
                Err(Box::new(order_error))
            }
        } else {
            let order_error = exchange::OrderError {
                msg: "wrong order type passed to submit".to_string(),
                code: 12 as i32,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    fn open_orders(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<exchange::Order> {
        let client = build_http_client(exchange).unwrap();
        let url = format!("{}/orders", exchange.api_url.as_str());
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

pub fn build_http_client(
    exchange: &config::ExchangeSettings,
) -> reqwest::Result<reqwest::blocking::Client> {
    let headers = header::HeaderMap::new();
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers)
        .build()
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

#[cfg(test)]
mod tests {
    use super::*;
    static PRIVKEY: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
    static GOOD_ADDR: &str = "ed6d484f5c289ec8c6b6f934ef6419230169f534";
    static MSG_V3: &str = "HYDRO-AUTHENTICATION@1524088776656";
    static GOOD_SIG_V3: &str = "603efd7241bfb6c61f4330facee0f7027d98e030ef241ad03a372638c317859a50620dacee177b771ce05812770a637c4c7395da0042c94250f86fb52472f9351b"; // was ..00"

    #[test]
    fn test_build_token() {
        let token = build_token(PRIVKEY, MSG_V3);
        let good_token = format!("0x{}#{}#0x{}", GOOD_ADDR, MSG_V3, GOOD_SIG_V3);
        assert_eq!(token, good_token);
    }
}
