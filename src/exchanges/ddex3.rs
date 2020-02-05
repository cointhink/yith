use crate::config;
use crate::types;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use secp256k1::{Secp256k1, SecretKey, PublicKey};
use hex::decode;
use tiny_keccak::{Keccak, Hasher};

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    Buy,
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LimitMarket {
    Limit,
    Market,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    market_id: String,
    side: BuySell,
    order_type: LimitMarket,
    price: f64,
    amount: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    status: i64,
    desc: String,
}

pub fn build(
    askbid: &types::AskBid,
    exchange: &config::ExchangeApi,
    market: &types::Market,
    offer: &types::Offer,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "HYDRO build {:#?} {} {}@{}",
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
        //wallet_type: "trading",
        price: price,
        amount: qty,
    };

    let privkey = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
    let client = build_auth_client(privkey)?;

    let url = exchange.build_url.as_str();
    println!("Ddex3 order {}", url);
    println!("{:#?}", sheet);

    let resp = client.post(url).json(&sheet).send()?;
    println!("{:#?}", resp);
    let body = resp.json::<BuildResponse>().unwrap();
    println!("{:#?}", body);
    Ok(())
}

pub fn build_auth_client(privkey: &str) -> reqwest::Result<reqwest::blocking::Client> {
    let mut secret = String::from("");
    let fixedtime = format!(
        "{}{}",
        "fixed",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    build_token(&mut secret, privkey, fixedtime.as_str());
    println!("token: {}", secret);
    let ddex_auth = "Hydro-Authentication";
    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::AUTHORIZATION,
        header::HeaderValue::from_str(secret.as_str()).unwrap(), //boom
    );

    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .default_headers(headers)
        .build()
}

fn build_token(token: &mut String, privkey: &str, msg: &str) {
    let secp = Secp256k1::new();
    let privbytes = &hex::decode(privkey).unwrap();
    let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let pubkey_bytes = &public_key.serialize_uncompressed();
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut output);
    let addr = &output[12..];  //.slice(-20)
    token.push_str(format!("{}#{}", hex::encode(addr), msg).as_str());
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
