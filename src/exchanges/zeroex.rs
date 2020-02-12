use crate::config;
use crate::eth;
use crate::types;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    r#type: BuySell,
    quantity: String,
    price: String,
    expiration: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderForm {
    chain_id: i64,
    maker_address: String,
    signature: String,
    sender_address: String,
    exchange_address: String,
    taker_address: String,
    maker_fee: String,
    taker_fee: String,
    maker_fee_asset_data: String,
    taker_fee_asset_data: String,
    maker_asset_amount: String,
    taker_asset_amount: String,
    maker_asset_data: String,
    taker_asset_data: String,
    salt: String,
    fee_recipient_address: String,
    expiration_time_seconds: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BuildResponse {
    status: i64,
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "BUY")]
    Buy,
    #[serde(rename = "SELL")]
    Sell,
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
    let mut ab = askbid;
    let askbid_other = askbid.otherside();
    if market.swapped {
        ab = &askbid_other;
        let (s_qty, s_price) = offer.swap();
        println!(
            "unswapped {:#?} {} {}-{} {}@{}",
            ab, market.source.name, market.quote, market.base, s_qty, s_price
        );
    }
    let side = match ab {
        types::AskBid::Ask => BuySell::Buy,
        types::AskBid::Bid => BuySell::Sell,
    };
    let expire_time = (SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
	+ std::time::Duration::new(120+5,0))  // 2min minimum + transittime
        .as_secs();
    let sheet = OrderSheet {
        r#type: side,
        quantity: format!("{}", qty),
        price: format!("{}", price),
        expiration: format!("{}", expire_time),
    };
    let url = format!(
        "{}/markets/{}/order/limit",
        exchange.api_url.as_str(),
        market_id
    );
    println!("0x order {}", url);
    println!("{:#?}", sheet);
    let client = reqwest::blocking::Client::new();
    let resp = client.post(url.as_str()).json(&sheet).send()?;
    println!("{:#?} {}", resp.status(), resp.url());
    if resp.status().is_success() {
        let mut form = resp.json::<OrderForm>().unwrap();
        println!("{:#?}", form);
        form.maker_address = format!("0x{}", eth::privkey_to_addr(privkey).to_string());
        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
        let eip712_hash =
            hex::decode("f80322eb8376aafb64eadf8f0d7623f22130fd9491a221e902b713cb984a7534")?;
        let tokens = &[
            ethabi::Token::FixedBytes(eip712_hash),
            ethabi::Token::FixedBytes(hex::encode(&form.maker_address).as_bytes().to_vec()),
            ethabi::Token::FixedBytes(hex::encode(&form.taker_address).as_bytes().to_vec()),
            ethabi::Token::FixedBytes(hex::encode(&form.fee_recipient_address).as_bytes().to_vec()),
            ethabi::Token::FixedBytes(hex::encode(&form.sender_address).as_bytes().to_vec()),
            ethabi::Token::Uint(primitive_types::U256::from(
                form.maker_asset_amount.parse::<u64>()?,
            )),
            ethabi::Token::Uint(primitive_types::U256::from(
                form.taker_asset_amount.parse::<u64>()?,
            )),
            ethabi::Token::Uint(primitive_types::U256::from(form.maker_fee.parse::<u64>()?)),
            ethabi::Token::Uint(primitive_types::U256::from(form.taker_fee.parse::<u64>()?)),
            ethabi::Token::Uint(primitive_types::U256::from(
                form.expiration_time_seconds.parse::<u64>()?,
            )),
            ethabi::Token::Uint(primitive_types::U256::from(form.salt.parse::<u64>()?)),
            ethabi::Token::Uint(primitive_types::U256::from(eth::hash_msg(
                &form.maker_asset_data.as_bytes().to_vec(),
            ))),
            ethabi::Token::Uint(primitive_types::U256::from(eth::hash_msg(
                &form.taker_asset_data.as_bytes().to_vec(),
            ))),
            ethabi::Token::Uint(primitive_types::U256::from(eth::hash_msg(
                &form.maker_fee_asset_data.as_bytes().to_vec(),
            ))),
            ethabi::Token::Uint(primitive_types::U256::from(eth::hash_msg(
                &form.taker_fee_asset_data.as_bytes().to_vec(),
            ))),
        ];
        let msg: Vec<u8> = ethabi::encode(tokens);
        let msg_hash = eth::hash_msg(&msg);
        let sig_bytes = eth::sign_bytes(&msg_hash, &secret_key);
        form.signature = format!("0x{}", hex::encode(&sig_bytes[..]));
        println!("filled in {:#?}", form);
        let url = format!("{}/orders", exchange.api_url.as_str());
        let resp = client.post(url.as_str()).json(&form).send()?;
        println!("{:#?} {}", resp.status(), resp.url());
        println!("{:#?}", resp.text());
    } else {
        let body = resp.json::<BuildResponse>().unwrap();
        println!("{:#?}", body);
    }

    Ok(())
}

pub fn order(os: OrderSheet) {
    println!("0x order! {:#?}", os);
}

pub fn make_market_id(swapped: bool, base: &types::Ticker, quote: &types::Ticker) -> String {
    match swapped {
        true => format!("{}-{}", quote.symbol, base.symbol),
        false => format!("{}-{}", base.symbol, quote.symbol),
    }
}
