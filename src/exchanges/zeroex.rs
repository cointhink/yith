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
    let expire_time = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap()
        + std::time::Duration::new(120 + 5, 0)) // 2min minimum + transittime
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
    println!("{}", url);
    let resp = client.post(url.as_str()).json(&sheet).send()?;
    println!("{:#?} {}", resp.status(), resp.url());
    if resp.status().is_success() {
        let mut form = resp.json::<OrderForm>().unwrap();
        form.maker_address = format!("0x{}", eth::privkey_to_addr(privkey).to_string());
        println!("{:#?}", form);
        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).expect("32 bytes, within curve order");
        let form_tokens = order_tokens(&form);
        let form_tokens_bytes: Vec<u8> = ethabi::encode(&form_tokens);
        let form_hash = eth::hash_msg(&form_tokens_bytes);
        let exg_tokens = exchange_order_tokens(form_hash, &form.exchange_address);
        let exg_tokens_bytes = ethabi::encode(&exg_tokens);
	let eip191_header = hex::decode("1901").unwrap();
	let exg_with_header = [&eip191_header[..], &exg_tokens_bytes[..]].concat();
	println!("exg_with_header {}", hex::encode(&exg_with_header));
        let exg_hash = eth::hash_msg(&exg_with_header);
        let form_sig_bytes = eth::sign_bytes(&exg_hash, &secret_key);
        form.signature = format!("0x{}", hex::encode(&form_sig_bytes[..]));
        println!("filled in {:#?}", form);
        let url = format!("{}/orders", exchange.api_url.as_str());
        println!("{}", url);
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

pub fn order_tokens(form: &OrderForm) -> Vec<ethabi::Token> {
    let eip712_order_schema_hash =
        hex::decode("f80322eb8376aafb64eadf8f0d7623f22130fd9491a221e902b713cb984a7534").unwrap();
    vec![
        ethabi::Token::FixedBytes(eip712_order_schema_hash),
        ethabi::Token::Address(str_to_H160(&form.maker_address[2..])),
        ethabi::Token::Address(str_to_H160(&form.taker_address[2..])),
        ethabi::Token::Address(str_to_H160(&form.fee_recipient_address[2..])),
        ethabi::Token::Address(str_to_H160(&form.sender_address[2..])),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.maker_asset_amount.parse::<u64>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.taker_asset_amount.parse::<u64>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.maker_fee.parse::<u64>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.taker_fee.parse::<u64>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.expiration_time_seconds.parse::<u64>().unwrap(),
        )),
        ethabi::Token::Uint(ethereum_types::U256::from(
            form.salt.parse::<u64>().unwrap(),
        )),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.maker_asset_data[2..])),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.taker_asset_data[2..])),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.maker_fee_asset_data[2..])),
        ethabi::Token::FixedBytes(hexstr_to_hashbytes(&form.taker_fee_asset_data[2..])),
    ]
}

pub fn str_to_H160(addr_str: &str) -> ethereum_types::H160 {
    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hex::decode(addr_str).unwrap());
    ethereum_types::H160::from(addr)
}

pub fn hexstr_to_hashbytes(msg_str: &str) -> Vec<u8> {
    eth::hash_msg(&hex::decode(&msg_str).unwrap()).to_vec()
}

pub fn exchange_order_tokens(order_hash: [u8; 32], contract_addr: &str) -> Vec<ethabi::Token> {
    let exchange_hash = eip712_exchange_hash(contract_addr);
    println!("exg hash {}", hex::encode(&exchange_hash));
    println!("ord hash {}", hex::encode(&order_hash));
    vec![
        ethabi::Token::FixedBytes(exchange_hash.to_vec()),
        ethabi::Token::FixedBytes(order_hash.to_vec()),
    ]
}

pub fn eip712_exchange_hash(contract_addr: &str) -> [u8; 32] {
    let eip712_domain_schema_hash =
        hex::decode("8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f").unwrap();
    let eip712_exchange_domain_name = "0x Protocol";
    let eip712_exchange_domain_version = "3.0.0";
    let chain_id = 1;
    let tokens = vec![
        ethabi::Token::FixedBytes(eip712_domain_schema_hash),
        ethabi::Token::FixedBytes(
            eth::hash_msg(&eip712_exchange_domain_name.as_bytes().to_vec()).to_vec(),
        ),
        ethabi::Token::FixedBytes(
            eth::hash_msg(&eip712_exchange_domain_version.as_bytes().to_vec()).to_vec(),
        ),
        ethabi::Token::Uint(ethereum_types::U256::from(chain_id)),
        ethabi::Token::Address(str_to_H160(&contract_addr[2..])),
    ];
    let token_bytes = ethabi::encode(&tokens);
    eth::hash_msg(&token_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    static contract_addr_v2: &str = "080bf510FCbF18b91105470639e9561022937712";
    static good_exchange_hash_v2: &str =
        "b2246130e7ae0d4b56269ccac10d3a9ac666d825bcd20ce28fea70f1f65d3de0";

    #[test]
    fn test_eip712_domain_sep() {
        let hash = eip712_exchange_hash(&format!("0x{}", contract_addr_v2));
        println!("edh hashbytes {}", hex::encode(&hash));
        assert_eq!(hash.to_vec(), hex::decode(good_exchange_hash_v2).unwrap())
    }

    #[test]
    fn test_order_tokens() {
        let form = OrderForm {
            chain_id: 1,
            maker_address: "0x0000000000000000000000000000000000000000".to_string(),
            taker_address: "0x0000000000000000000000000000000000000000".to_string(),
            fee_recipient_address: "0x0000000000000000000000000000000000000000".to_string(),
            sender_address: "0x0000000000000000000000000000000000000000".to_string(),
            maker_asset_amount: "0".to_string(),
            taker_asset_amount: "0".to_string(),
            maker_fee: "0".to_string(),
            taker_fee: "0".to_string(),
            expiration_time_seconds: "0".to_string(),
            salt: "0".to_string(),
            exchange_address: contract_addr_v2.to_string(),
            maker_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            taker_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            maker_fee_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            taker_fee_asset_data: "0x0000000000000000000000000000000000000000".to_string(),
            signature: "SET".to_string(),
        };
        let form_tokens = order_tokens(&form);
        let form_tokens_bytes: Vec<u8> = ethabi::encode(&form_tokens);
        println!("form_tokens_bytes {}", hex::encode(&form_tokens_bytes));
        let good_form_tokens_bytes = "f80322eb8376aafb64eadf8f0d7623f22130fd9491a221e902b713cb984a753400000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000005380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a";
        assert_eq!(
            form_tokens_bytes,
            hex::decode(good_form_tokens_bytes).unwrap()
        );
        let form_hash = eth::hash_msg(&form_tokens_bytes);
        let good_form_hash = "6272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        assert_eq!(form_hash.to_vec(), hex::decode(good_form_hash).unwrap());
    }

    #[test]
    fn test_exchange_tokens() {
        let mut form_hash = [0u8; 32];
        let good_form_hash = "6272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        form_hash.copy_from_slice(&hex::decode(good_form_hash).unwrap());
        let tokens = exchange_order_tokens(form_hash, &format!("0x{}", contract_addr_v2));
	println!("tokens len {}", tokens.len());
        let exchange_tokens_bytes = ethabi::encode(&tokens);
	let eip191_header = hex::decode("1901").unwrap();
	let exg_with_header = [&eip191_header[..], &exchange_tokens_bytes[..]].concat();
        println!(
            "exchange_tokens_bytes {}",
            hex::encode(&exg_with_header)
        );
        let good_exchange_tokens_bytes = "1901b2246130e7ae0d4b56269ccac10d3a9ac666d825bcd20ce28fea70f1f65d3de06272bc49657b2210a4eba2cd343aa184ed1b77c377cad3b452afa50be0f15d06";
        println!("Gxchange_tokens_bytes {}", good_exchange_tokens_bytes);
        assert_eq!(
            exg_with_header,
            hex::decode(good_exchange_tokens_bytes).unwrap()
        );
    }

    #[test]
    fn test_hexstr_to_hashbytes() {
        assert_eq!(
            hexstr_to_hashbytes(&"0x0000000000000000000000000000000000000000"[2..]),
            hex::decode("5380c7b7ae81a58eb98d9c78de4a1fd7fd9535fc953ed2be602daaa41767312a")
                .unwrap()
        );
        assert_eq!(
            hexstr_to_hashbytes(&"0x"[2..]),
            hex::decode("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470")
                .unwrap()
        )
    }
}
