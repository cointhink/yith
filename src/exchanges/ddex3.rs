use crate::config;
use crate::types;
use reqwest::header;
use reqwest::Proxy;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_keccak::{Hasher, Keccak};

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
pub struct OrderSheet {
    #[serde(rename = "marketId")]
    market_id: String,
    side: BuySell,
    #[serde(rename = "orderType")]
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
    privkey: &str,
    askbid: &types::AskBid,
    exchange: &config::ExchangeApi,
    market: &types::Market,
    offer: &types::Offer,
    proxy: &str,
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

    let client = build_auth_client(proxy)?;

    let url = exchange.build_url.as_str();
    println!("Ddex3 order {}", url);
    println!("{:#?}", sheet);

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
    let resp = client.post(url).headers(headers).json(&sheet).send()?;
    println!("{:#?} {}", resp.status(), resp.url());
    let body = resp.json::<BuildResponse>().unwrap();
    println!("{:#?}", body);
    Ok(())
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
    let pubkey_bytes = &public_key.serialize_uncompressed();
    let addr = pubkey_to_addr(pubkey_bytes);
    let mut msg_hash = [0u8; 32];
    hash_msg(&mut msg_hash, msg);
    let sig_bytes = sign_bytes(&msg_hash, secret_key);
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

pub fn pubkey_to_addr(pubkey_bytes: &[u8; 65]) -> [u8; 20] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut output);
    let mut sized_output = [0u8; 20];
    sized_output.copy_from_slice(&output[12..32]);
    sized_output
}

pub fn hash_msg(mut msg_hash: &mut [u8], msg: &str) {
    let hash_full = format!("\u{0019}Ethereum Signed Message:\n{}{}", msg.len(), msg);
    let mut hasher = Keccak::v256();
    hasher.update(hash_full.as_bytes());
    hasher.finalize(&mut msg_hash);
}

pub fn sign_bytes(msg_hash: &[u8], secret_key: SecretKey) -> [u8; 65] {
    let secp = Secp256k1::new();
    let secp_msg = Message::from_slice(&msg_hash).unwrap();
    let signature = secp.sign_recoverable(&secp_msg, &secret_key);
    let (recovery_id, sig) = signature.serialize_compact();
    let mut vec = Vec::with_capacity(65);
    vec.extend_from_slice(&sig);
    // chainId + 27
    let r = recovery_id.to_i32() + 27;
    vec.push(r as u8);
    let mut sig_sized_bytes = [0u8; 65];
    sig_sized_bytes.copy_from_slice(vec.as_slice());
    sig_sized_bytes
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
    static pubkey: &str = "041ea3510efdb57c6cf0dc77a454b4f5b95775f9606b0f7d7a294b47aae57b21882e6c4888d050992b58a0640066ab72adff7575c07d201716c40b9146624eedb4";
    static good_addr: &str = "ed6d484f5c289ec8c6b6f934ef6419230169f534";
    static msg_v4: &str = "HYDRO-AUTHENTICATION@1566380397473";
    static msg_v3: &str = "HYDRO-AUTHENTICATION@1524088776656";
    static good_sig_v4: &str = "2a10e17a0375a6728947ae4a4ad0fe88e7cc8dd929774be0e33d7e1988f1985f13cf66267134ec4777878b6239e7004b9d2defb03ede94352a20acf0a20a50dc1b";
    static good_sig_v3: &str = "603efd7241bfb6c61f4330facee0f7027d98e030ef241ad03a372638c317859a50620dacee177b771ce05812770a637c4c7395da0042c94250f86fb52472f93500";

    #[test]
    fn test_pubkey_to_addr() {
        let pubkey_bytes = hex::decode(pubkey).unwrap();
        let mut pubkey_sized_bytes = [0u8; 65];
        pubkey_sized_bytes.copy_from_slice(&pubkey_bytes);
        let addr_bytes = pubkey_to_addr(&pubkey_sized_bytes);
        let addr = hex::encode(addr_bytes);
        assert_eq!(addr, good_addr);
    }

    #[test]
    fn test_hash_msg() {
        //let good_hash_v4 = "68cef504a5bf9b821df3313da9af66354d8865f29ba038c42b62cea53cd9986d";
        let good_hash_v3 = "14d10d289a1662f15e85ddc809acf1f89a888dda71ddaacb1deb60113f6d310f";
        let good_hash_bytes = hex::decode(good_hash_v3).unwrap();
        let mut good_hash_sized_bytes = [0u8; 32];
        good_hash_sized_bytes.copy_from_slice(&good_hash_bytes);
        let mut hash_bytes = [0u8; 32];
        hash_msg(&mut hash_bytes, msg_v3);
        assert_eq!(hash_bytes, good_hash_sized_bytes);
    }

    #[test]
    fn test_sign_bytes() {
        let hash_v4: &[u8] = b"68cef504a5bf9b821df3313da9af66354d8865f29ba038c42b62cea53cd9986d";
        let hash_v3: &[u8] = b"14d10d289a1662f15e85ddc809acf1f89a888dda71ddaacb1deb60113f6d310f";
        let hash_bytes: Vec<u8> = hex::decode(hash_v4).unwrap();
        let privkey_bytes: Vec<u8> = hex::decode(privkey).unwrap();
        let private_key =
            SecretKey::from_slice(&privkey_bytes).expect("32 bytes, within curve order");
        let sig_bytes = sign_bytes(&hash_bytes, private_key);
        let good_sig_bytes: Vec<u8> = hex::decode(good_sig_v4).unwrap();
        let mut good_sig_sized_bytes = [0u8; 65];
        good_sig_sized_bytes.copy_from_slice(&good_sig_bytes);
        assert_eq!(&sig_bytes[..], &good_sig_sized_bytes[..]);
    }

    #[test]
    fn test_build_token() {
        let mut token = String::from("");
        build_token(&mut token, privkey, msg_v4);
        let good_token = format!("0x{}#{}#0x{}", good_addr, msg_v4, good_sig_v4);
        assert_eq!(token, good_token);
    }
}
