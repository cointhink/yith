use crate::config;
use crate::types;
use reqwest::header;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};
use tiny_keccak::{Hasher, Keccak};

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
    println!("pubkey: {:?}", hex::encode(&pubkey_bytes[..]));
    let addr = pubkey_to_addr(pubkey_bytes);
    let mut msg_hash = [0u8; 32];
    hash_msg(&mut msg_hash, msg, pubkey_bytes);
    let sig_bytes = sign_bytes(&msg_hash, secret_key);
    println!("sig: {:?}", hex::encode(&sig_bytes[..]));
    let signed_hash = msg_hash;
    token.push_str(
        format!(
            "0x{}#{}#0x{}",
            hex::encode(addr),
            msg,
            hex::encode(signed_hash)
        )
        .as_str(),
    );
}

pub fn pubkey_to_addr(pubkey_bytes: &[u8; 65]) -> [u8; 20] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut output);
    let mut sized_output = [0u8;20];
    sized_output.copy_from_slice(&output[12..32]);
    sized_output
}

pub fn hash_msg(mut msg_hash: &mut [u8], msg: &str, pubkey_bytes: &[u8; 65]) {
    let hash_full = format!("\u{0019}Ethereum Signed Message:\n{}{}", msg.len(), msg);
    let mut hasher = Keccak::v256();
    hasher.update(&pubkey_bytes[1..]);
    hasher.finalize(&mut msg_hash);
}

pub fn sign_bytes(msg_hash: &[u8], secret_key: SecretKey) -> [u8;64] {
    let secp2 = Secp256k1::new();
    let scmsg = Message::from_slice(&msg_hash).unwrap();
    let sig = secp2.sign(&scmsg, &secret_key);
    //let (recovery_id, serialize_sig) = sig.serialize_compact();
    sig.serialize_compact()
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

    #[test]
    fn test_pubkey_to_addr() {
        let pubkey = "041ea3510efdb57c6cf0dc77a454b4f5b95775f9606b0f7d7a294b47aae57b21882e6c4888d050992b58a0640066ab72adff7575c07d201716c40b9146624eedb4";
        let pubkey_bytes = hex::decode(pubkey).unwrap();
        let mut pubkey_sized_bytes = [0u8; 65];
        pubkey_sized_bytes.copy_from_slice(&pubkey_bytes);
        let addr_bytes = pubkey_to_addr(&pubkey_sized_bytes);
        let addr = hex::encode(addr_bytes);
        let good_addr = "ed6d484f5c289ec8c6b6f934ef6419230169f534";
        assert_eq!(addr, good_addr);
    }

    #[test]
    fn test_build_token() {
        let mut token = String::from("");
        let privkey = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";
        let msg = "HYDRO-AUTHENTICATION@1566380397473";
        build_token(&mut token, privkey, msg);
        let good_auth = "ed6d484f5c289ec8c6b6f934ef6419230169f534#HYDRO-AUTHENTICATION@1566380397473#0x2a10e17a0375a6728947ae4a4ad0fe88e7cc8dd929774be0e33d7e1988f1985f13cf66267134ec4777878b6239e7004b9d2defb03ede94352a20acf0a20a50dc1b";
        assert_eq!(token, good_auth);
    }
}
