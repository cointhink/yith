use crate::config;
use crate::eth;
use crate::etherscan;
use crate::exchange;
use crate::exchange::Api;
use crate::geth;
use crate::http;
use crate::time;
use crate::types;
use reqwest::header;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::Duration;

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSheet {
    orders: Vec<OrderSheetOrder>,
    starting_nonce: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderSheetOrder {
    order_hash: String,
    amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheetSignedOrder {
    #[serde(flatten)]
    order_sheet: OrderSheetOrder,
    nonce: String,
    address: String,
    v: u8,
    r: String,
    s: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawRequest {
    address: String,
    amount: String,
    token: String,
    nonce: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawRequestSigned {
    #[serde(flatten)]
    withdraw_request: WithdrawRequest,
    v: u8,
    r: String,
    s: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    #[serde(flatten)]
    balances: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryRequest {
    address: String,
    start: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistoryResponse {
    trades: HashMap<String, Vec<TradeHistory>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TradeHistory {
    r#type: String,
    date: String,
    amount: String,
    total: String,
    uuid: String,
    tid: u32,
    timestamp: u32,
    price: String,
    taker: String,
    maker: String,
    order_hash: String,
    transaction_hash: String,
    token_buy: String,
    buyer_fee: String,
    gas_fee: String,
    seller_fee: String,
    token_sell: String,
    usd_value: String,
}

impl TradeHistory {
    fn into_exg(self, buy: &str, sell: &str) -> exchange::Order {
        exchange::Order {
            id: self.tid.to_string(), //self.uuid,
            side: match self.r#type.as_str() {
                "buy" => exchange::BuySell::Buy,
                "sell" => exchange::BuySell::Sell,
                _ => panic!(),
            },
            state: exchange::OrderState::Filled,
            market: format!("{}/{}", buy, sell),
            base_qty: self.amount.parse::<f64>().unwrap(),
            quote: self.price.parse::<f64>().unwrap(),
            create_date: self.date,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusRequest {
    order_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    date: String,
    market: String,
    r#type: String,
    price: String,
    amount: String,
    total: String,
    order_hash: String,
    uuid: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusResponse {
    status: String,
    market: String,
    r#type: String,
}

impl Into<exchange::OrderState> for OrderStatusResponse {
    fn into(self) -> exchange::OrderState {
        match self.status.as_ref() {
            "open" => exchange::OrderState::Open,
            "cancelled" => exchange::OrderState::Cancelled,
            _ => panic!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NonceResponse {
    nonce: u128, //docs wrong
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorResponse {
    error: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookRequest {
    market: String,
    count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookResponse {
    bids: Vec<OrderBookEntry>,
    asks: Vec<OrderBookEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderBookEntry {
    price: String,
    total: String,
    order_hash: String,
    amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDetail {
    pub name: String,
    pub address: String,
    pub decimals: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenList {
    #[serde(flatten)]
    tokens: HashMap<String, TokenDetail>,
}

impl TokenList {
    pub fn read_tokens(filename: &str) -> TokenList {
        let file_ok = fs::read_to_string(filename);
        let yaml = file_ok.unwrap();
        let tokens = serde_yaml::from_str(&yaml).unwrap();
        TokenList { tokens: tokens }
    }

    pub fn get(&self, symbol2: &str) -> &TokenDetail {
        let detail = self
            .tokens
            .iter()
            .find(|(symbol, _detail)| *symbol == symbol2)
            .unwrap()
            .1;
        // println!(
        //     "idex lookup {} ^{} {}",
        //     symbol2, detail.decimals, detail.address
        // );
        detail
    }

    pub fn by_addr(&self, addr: &str) -> (&String, &TokenDetail) {
        let detail = self
            .tokens
            .iter()
            .find(|(_symbol, detail)| detail.address == addr)
            .unwrap();
        // println!(
        //     "idex by_addr {} ^{} {}",
        //     detail.0, detail.1.decimals, detail.1.address
        // );
        detail
    }
}

#[allow(dead_code)]
pub struct Idex {
    geth: geth::Client,
    settings: config::ExchangeSettings,
    client: http::LoggingClient,
    tokens: TokenList,
}

impl Idex {
    pub fn new(settings: config::ExchangeSettings, api_key: &str, geth: geth::Client) -> Idex {
        let client = Idex::build_http_client(api_key).unwrap();
        let logging_client = http::LoggingClient::new(client);
        let tokens = TokenList::read_tokens("notes/idex-tokens.json");
        log::debug!("idex loaded {} tokens", tokens.tokens.len());
        Idex {
            geth: geth,
            settings: settings,
            client: logging_client,
            tokens: tokens,
        }
    }

    pub fn build_http_client(api_key: &str) -> reqwest::Result<reqwest::blocking::Client> {
        let mut headers = header::HeaderMap::new();
        headers.insert(
            "API-Key",
            header::HeaderValue::from_str(api_key).unwrap(), //boom
        );
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .default_headers(headers)
            .build()
    }

    pub fn nonce(&self, privkey: &str) -> usize {
        let pub_addr = eth::privkey_to_addr(privkey);
        let url = format!(
            "{}/returnNextNonce?address=0x{}",
            self.settings.api_url.as_str(),
            pub_addr
        );
        let resp = self.client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        println!("{} {}", url, status);
        let nonce = resp.json::<NonceResponse>().unwrap().nonce;
        nonce as usize
    }

    pub fn balance_wait(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
        token: &str,
    ) {
        println!("idex transfer stage 2 balance watch {}", token);
        let old_balance: f64 = *self
            .balances(public_addr, exchange)
            .get(token)
            .unwrap_or(&0.0);
        let start = time::now();
        let mut same = true;
        while same {
            let balances = self.balances(public_addr, exchange);
            same = match balances.get(token) {
                Some(balance) => {
                    let waited = start.elapsed();
                    println!(
                        "idex balance {} => {} {} {}",
                        old_balance,
                        balance,
                        token,
                        time::duration_words(waited)
                    );
                    if *balance == old_balance {
                        time::sleep(10000);
                        true
                    } else {
                        false // not the same
                    }
                }
                None => true,
            };
        }
    }
}

impl exchange::Api for Idex {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
        let base_token = self.tokens.get(&market.base.symbol);
        let quote_token = self.tokens.get(&market.quote.symbol);
        let nonce = self.nonce(privkey); // call before OrderBook #speed

        let url = format!("{}/returnOrderBook", exchange.api_url.as_str(),);
        let market_name = format!("{}_{}", &market.quote.symbol, &market.base.symbol);
        let order_book_request = OrderBookRequest {
            market: market_name.clone(),
            count: 2,
        };
        let resp = self
            .client
            .post(url.as_str())
            .json(&order_book_request)
            .send()
            .unwrap();
        let json = resp.text().unwrap();
        let book = serde_json::from_str::<OrderBookResponse>(&json).unwrap();
        let side = match askbid {
            types::AskBid::Ask => book.asks,
            types::AskBid::Bid => book.bids,
        };
        println!("{} {:?} orderbook snapshot {:?}", market_name, askbid, side);

        let buy_token = match askbid {
            types::AskBid::Ask => quote_token,
            types::AskBid::Bid => base_token,
        };

        let mut orders = vec![];
        let buy_qty = match askbid {
            types::AskBid::Ask => offer.cost(types::AskBid::Ask),
            types::AskBid::Bid => offer.base_qty,
        };
        let mut remaining_buy = buy_qty;
        side.iter().for_each(|o| {
            let price = o.price.parse::<f64>().unwrap();
            let qty = o.amount.parse::<f64>().unwrap();
            let cost = match askbid {
                types::AskBid::Ask => price * qty,
                types::AskBid::Bid => qty,
            };
            if price <= offer.quote {
                let min_buy = eth::minimum(&vec![remaining_buy, cost]);
                let amount = exchange::quantity_in_base_units(
                    remaining_buy,
                    buy_token.decimals,
                    buy_token.decimals,
                );
                let order = OrderSheetOrder {
                    order_hash: o.order_hash.clone(),
                    amount: amount.to_str_radix(10),
                };
                if min_buy > 0.0 {
                    orders.push(order);
                }
                remaining_buy -= min_buy;
                println!(
                    "+ {:0.5}@{:0.5}={:0.5} {:0.5}@{:0.5}={:0.5} spending {:0.5}{} remaining {:0.5}",
                    offer.base_qty,
                    offer.quote,
                    buy_qty,
                    qty,
                    price,
                    cost,
                    min_buy,
                    buy_token.name,
                    remaining_buy,
                );
            }
        });

        if orders.len() > 0 {
            Ok(exchange::OrderSheet::Idex(OrderSheet {
                orders: orders,
                starting_nonce: nonce,
            }))
        } else {
            Err(exchange::ExchangeError::build_box(format!(
                "No offers availble to match"
            )))
        }
    }

    fn submit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if let exchange::OrderSheet::Idex(order_sheet) = sheet {
            let privbytes = &hex::decode(privkey).unwrap();
            let secret_key = SecretKey::from_slice(privbytes).unwrap();

            let address = format!("0x{}", eth::privkey_to_addr(privkey));
            let mut orders: Vec<OrderSheetSignedOrder> = vec![];
            let starting_nonce = order_sheet.starting_nonce;
            order_sheet
                .orders
                .into_iter()
                .enumerate()
                .for_each(|(idx, o)| {
                    let order_nonce = (starting_nonce + idx).to_string();
                    let order_hash_bytes = trade_params_hash(&o, &address, &order_nonce);
                    let order_hash = eth::ethsign_hash_msg(&order_hash_bytes[..].to_vec());
                    let (v, r, s) = eth::sign_bytes_vrs(&order_hash, &secret_key);
                    let so = OrderSheetSignedOrder {
                        order_sheet: o,
                        nonce: order_nonce,
                        address: address.clone(),
                        r: eth::hex(&r),
                        s: eth::hex(&s),
                        v: v,
                    };
                    orders.push(so);
                });
            println!("{}", serde_json::to_string(&orders).unwrap());
            let url = format!("{}/trade", exchange.api_url.as_str());
            let resp = self.client.post(url.as_str()).json(&orders).send().unwrap();
            println!("{} {}", url, resp.status());
            if resp.status().is_success() {
                let json = resp.text().unwrap();
                println!("{}", json);
                let orders = serde_json::from_str::<Vec<OrderResponse>>(&json).unwrap();
                // TODO handle multiple orders
                if orders.len() > 1 {
                    println!(
                        "warning: {} orders in play {}",
                        orders.len(),
                        orders
                            .iter()
                            .map(|o| o.uuid.as_str())
                            .collect::<Vec<&str>>()
                            .join(", ")
                    )
                }
                Ok(orders[0].uuid.clone())
            } else {
                let json = resp.text().unwrap();
                let response = serde_json::from_str::<ErrorResponse>(&json).unwrap();
                Err(exchange::ExchangeError::build_box(response.error))
            }
        } else {
            Err(exchange::ExchangeError::build_box(format!(
                "wrong ordersheet type!"
            )))
        }
    }

    fn balances<'a>(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceList {
        let url = format!(
            "{}/returnBalances?address=0x{}",
            exchange.api_url.as_str(),
            public_addr
        );
        let resp = self.client.get(url.as_str()).send().unwrap();
        let response = resp.json::<BalanceResponse>().unwrap();
        response
            .balances
            .iter()
            .map(|(symbol, strval)| {
                let f64 = exchange::str_to_chopped_f64(strval);
                (symbol.clone(), f64)
            })
            .collect()
    }

    fn withdraw(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let url = format!("{}/withdraw", exchange.api_url.as_str());
        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let nonce = self.nonce(private_key);
        let base_token = &self.tokens.get(&token.symbol);
        let bigint = exchange::quantity_in_base_units(amount, 18, 18);
        let withdraw = WithdrawRequest {
            address: pub_addr,
            amount: bigint.to_str_radix(10),
            token: base_token.address.clone(),
            nonce: nonce.to_string(),
        };
        let params_hash_bytes = withdraw_params_hash(&withdraw, &exchange.contract_address);
        let params_hash = eth::ethsign_hash_msg(&params_hash_bytes[..].to_vec());
        let private_key_bytes = &hex::decode(private_key).unwrap();
        let secret_key = SecretKey::from_slice(private_key_bytes).unwrap();
        let (v, r, s) = eth::sign_bytes_vrs(&params_hash, &secret_key);
        let signed = WithdrawRequestSigned {
            withdraw_request: withdraw,
            v: v,
            r: eth::hex(&r),
            s: eth::hex(&s),
        };
        let last_blk = self.geth.last_block().to_string(); // save for later
        let resp = self.client.post(url.as_str()).json(&signed).send().unwrap();
        let status = resp.status();
        if status.is_success() {
            let json = resp.text().unwrap();
            //{"amount":"0","timestamp":null} no useful info
            Ok(Some(format!("{}.{}", token.symbol, last_blk)))
        } else {
            let json = resp.text().unwrap();
            let response = serde_json::from_str::<ErrorResponse>(&json).unwrap();
            Err(exchange::ExchangeError::build_box(response.error))
        }
    }

    fn deposit(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        ticker: &types::Ticker,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        println!("depositing {} amount {}", ticker.symbol, amount);
        let (data, value) = if ticker.symbol == "ETH" {
            let bigint = exchange::quantity_in_base_units(amount, 18, 18);
            (
                deposit_data(),
                ethereum_types::U256::from_dec_str(&bigint.to_str_radix(10)).unwrap(),
            )
        } else {
            let token = &self.tokens.get(&ticker.symbol);
            let bigint = exchange::quantity_in_base_units(amount, token.decimals, token.decimals);
            (
                deposit_token_data(&token.address, &bigint.to_str_radix(10)),
                ethereum_types::U256::zero(),
            )
        };

        let pub_addr = format!("0x{}", eth::privkey_to_addr(private_key));
        let nonce = self.geth.nonce(&pub_addr).unwrap();
        let gas_price_fast = geth::ethgasstation_fast();
        let gas_price_gwei = gas_price_fast / 1_000_000_000u64;
        println!("deposit tx gas {}gwei (ethgasstation_fast)", gas_price_gwei);

        let mut contract_addra = [0u8; 20];
        let contract_addr = exchange.contract_address.clone();
        contract_addra.copy_from_slice(&eth::dehex(&contract_addr)[..]);
        let tx = ethereum_tx_sign::RawTransaction {
            nonce: ethereum_types::U256::from(nonce),
            to: Some(ethereum_types::H160::from(contract_addra)),
            value: value,
            gas_price: ethereum_types::U256::from(gas_price_fast),
            gas: ethereum_types::U256::from(310240),
            data: data,
        };
        let private_key = ethereum_types::H256::from_slice(&eth::dehex(private_key));
        let rlp_bytes = tx.sign(&private_key, &eth::ETH_CHAIN_MAINNET);
        let params = (eth::hex(&rlp_bytes),);
        let tx = self
            .geth
            .rpc_str("eth_sendRawTransaction", geth::ParamTypes::Single(params))?;
        println!("GOOD TX {}", tx);
        Ok(Some(format!("{}.{}", ticker.symbol, tx)))
    }

    fn open_orders(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<exchange::Order> {
        let public_addr = eth::privkey_to_addr(private_key);
        let url = format!("{}/returnTradeHistoryMeta", exchange.api_url.as_str());
        let order_status = TradeHistoryRequest {
            address: public_addr,
            start: 1000,
        };
        let resp = self
            .client
            .post(url.as_str())
            .json(&order_status)
            .send()
            .unwrap();
        let status = resp.status();
        let json = resp.text().unwrap();
        let response = serde_json::from_str::<TradeHistoryResponse>(&json).unwrap();
        let mut orders = vec![];
        response.trades.into_iter().for_each(|(mkt, ths)| {
            let parts: Vec<&str> = mkt.split("_").collect();
            ths.into_iter()
                .for_each(|th| orders.push(th.into_exg(parts[0], parts[1])))
        });
        orders
    }

    fn order_status(
        &self,
        order_id: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::OrderState {
        if order_id.len() == 36 {
            // uuid from TradeHistory
            println!("uuid order status assumed filled!");
            exchange::OrderState::Filled
        } else {
            // order hash, len 67
            let url = format!("{}/returnOrderStatus", exchange.api_url.as_str());
            let order_status = OrderStatusRequest {
                order_hash: order_id.to_string(),
            };
            let resp = self
                .client
                .post(url.as_str())
                .json(&order_status)
                .send()
                .unwrap();
            let status = resp.status();
            let json = resp.text().unwrap();
            println!("{} {} {:?}", url, status, json);
            let response = serde_json::from_str::<OrderStatusResponse>(&json).unwrap();
            response.into()
        }
    }

    fn transfer_status<'a>(
        &self,
        transfer_id: &str,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceStatus {
        let parts: Vec<&str> = transfer_id.split(".").collect();
        let token = parts[0];
        let transfer_tx = parts[1];
        if transfer_tx.len() == 66 {
            // deposit tx
            let params = (transfer_tx.to_string(),);
            let result = self
                .geth
                .rpc(
                    "eth_getTransactionReceipt",
                    geth::ParamTypes::Single(params),
                )
                .unwrap();
            match result.part {
                geth::RpcResultTypes::Error(e) => {
                    println!("{}", e.error.message);
                    exchange::BalanceStatus::Complete
                }
                geth::RpcResultTypes::Result(r) => match r.result {
                    geth::ResultTypes::TransactionReceipt(tr) => {
                        match u32::from_str_radix(&tr.status[2..], 16).unwrap() {
                            1 => {
                                self.balance_wait(public_addr, exchange, token);
                                exchange::BalanceStatus::Complete
                            }
                            _ => {
                                println!("deposit tx failed. erc20 allowance problem?");
                                exchange::BalanceStatus::Error
                            }
                        }
                    }
                    _ => exchange::BalanceStatus::InProgress,
                },
            }
        } else {
            // withdrawal transfer_id is last_blocknumber
            let transfer_block_num = parts[1].parse::<u64>().unwrap();
            let config = config::CONFIG.get().unwrap();
            const TRANSFER_CONTRACT: &'static str = "0x2a0c0dbecc7e4d658f48e01e3fa353f44050c208";
            match token {
                "ETH" => {
                    match etherscan::last_internal_transaction_from(
                        public_addr,
                        transfer_block_num,
                        TRANSFER_CONTRACT,
                        &config.etherscan_key,
                    ) {
                        Ok(_intx) => exchange::BalanceStatus::Complete,
                        Err(_e) => exchange::BalanceStatus::InProgress,
                    }
                }
                _ => {
                    match etherscan::last_token_transaction(
                        public_addr,
                        transfer_block_num,
                        token,
                        &config.etherscan_key,
                    ) {
                        Ok(_erctx) => exchange::BalanceStatus::Complete,
                        Err(_e) => exchange::BalanceStatus::InProgress,
                    }
                }
            }
        }
    }
}

pub fn deposit_token_data(token_address: &str, amount: &str) -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("depositToken(address,uint256)").to_vec();
    call.append(&mut func);
    let mut p1 = hex::decode(eth::encode_addr2(token_address)).unwrap();
    call.append(&mut p1);
    let mut p2 = hex::decode(eth::encode_uint256(amount)).unwrap();
    call.append(&mut p2);
    call
}

pub fn deposit_data() -> Vec<u8> {
    let mut call = Vec::<u8>::new();
    let mut func = eth::hash_abi_sig("deposit()").to_vec();
    call.append(&mut func);
    call
}

pub fn withdraw_params_hash(wd: &WithdrawRequest, contract_address: &str) -> [u8; 32] {
    let parts: Vec<Vec<u8>> = vec![
        encode_addr(contract_address),
        encode_addr(&wd.token),
        eth::encode_uint256(&wd.amount),
        encode_addr(&wd.address),
        eth::encode_uint256(&wd.nonce),
    ];
    parts_hash(parts)
}

pub fn trade_params_hash(order: &OrderSheetOrder, address: &str, nonce: &str) -> [u8; 32] {
    let parts: Vec<Vec<u8>> = vec![
        encode_addr(&order.order_hash),
        eth::encode_uint256(&order.amount),
        encode_addr(address),
        eth::encode_uint256(nonce),
    ];
    parts_hash(parts)
}

// Idex uses unpadded value for addresses
pub fn encode_addr(str: &str) -> Vec<u8> {
    // 160bits/20bytes
    let hexletters = str[2..].to_lowercase();
    hexletters.as_bytes().to_vec()
}

pub fn parts_hash(mut parts: Vec<Vec<u8>>) -> [u8; 32] {
    let hash_hex = parts.iter_mut().fold(Vec::<u8>::new(), |mut memo, part| {
        memo.append(part);
        memo
    });
    let hashes = hex::decode(&hash_hex).unwrap();
    eth::hash_msg(&hashes)
}

#[cfg(test)]
mod tests {
    use super::*;

    static PRIVKEY_DDEX3: &str = "e4abcbf75d38cf61c4fde0ade1148f90376616f5233b7c1fef2a78c5992a9a50";

    #[test]
    fn test_order_params_hash() {
        let address = eth::privkey_to_addr(PRIVKEY_DDEX3);

        /*
          "tokenBuy": "0x0000000000000000000000000000000000000000",
          "amountBuy": "150000000000000000",
          "tokenSell": "0xcdcfc0f66c522fd086a1b725ea3c0eeb9f9e8814",
          "amountSell": "1000000000000000000000",
          "address": "0xed6d484f5c289ec8c6b6f934ef6419230169f534",
          "nonce": 123,
          "expires": 100000,
        */
        // let order_sheet = OrderSheet {
        //     token_buy: "0x0000000000000000000000000000000000000000".to_string(), //market.base_contract.clone(),
        //     amount_buy: "150000000000000000".to_string(),
        //     token_sell: "0xcdcfc0f66c522fd086a1b725ea3c0eeb9f9e8814".to_string(), //market.quote_contract.clone(),
        //     amount_sell: "1000000000000000000000".to_string(),
        //     address: format!("0x{}", address),
        //     nonce: 123.to_string(),
        //     expires: 100000,
        // };
        let idex_contract = "0x2a0c0dbecc7e4d658f48e01e3fa353f44050c208";
        // let order_hash_bytes = order_params_hash(&order_sheet, idex_contract);
        let good_hash = "0x385777b82d67f8368848ccd56f6ad04159bb6fc1075ae06910abb597c5a7c6a0";
        // assert_eq!(good_hash[2..], hex::encode(order_hash_bytes));
    }

    #[test]
    fn test_order_params_sign() {
        let privbytes = &hex::decode(PRIVKEY_DDEX3).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();

        let order_hash_str = "0x385777b82d67f8368848ccd56f6ad04159bb6fc1075ae06910abb597c5a7c6a0";
        let order_params_hash = hex::decode(&order_hash_str[2..]).unwrap();
        let order_hash = eth::ethsign_hash_msg(&order_params_hash[..].to_vec());
        let (_v, r, s) = eth::sign_bytes_vrs(&order_hash, &secret_key);

        let good_r = "0x860874c6d650c646389e3a7fbcd835665e546cbafa9831438d3a71535c19c50f";
        let good_s = "0x18205ecf4a6927e8653828c5508c3676f634c74051d9ef4f9216dbef43594a25";

        assert_eq!(hex::encode(r), good_r[2..]);
        assert_eq!(hex::encode(s), good_s[2..]);
    }
}
