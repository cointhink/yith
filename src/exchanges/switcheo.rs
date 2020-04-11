use crate::config;
use crate::eth;
use crate::exchange;
use crate::exchange::Api;
use crate::time;
use crate::types;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
use std::cell;
use std::collections::HashMap;
use std::fmt::Display;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair {
    name: String,
    precision: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PairList {
    pairs: Vec<Pair>,
}

impl PairList {
    pub fn get(&self, market: &str) -> Option<&Pair> {
        let mut result: Option<&Pair> = None;
        for pair in &self.pairs {
            if pair.name == market {
                result = Some(&pair)
            }
        }
        result
    }

    pub fn len(&self) -> usize {
        self.pairs.len()
    }
}

pub fn read_pairs(filename: &str) -> PairList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let pairs = serde_yaml::from_str::<Vec<Pair>>(&yaml).unwrap();
    PairList { pairs: pairs }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TokenDetail {
    pub symbol: String,
    name: String,
    r#type: String,
    hash: String,
    pub decimals: i32,
    transfer_decimals: i32,
    precision: i32,
    minimum_quantity: String,
    trading_active: bool,
    is_stablecoin: bool,
    stablecoin_type: Option<String>,
}

pub struct TokenList {
    pub tokens: HashMap<String, TokenDetail>,
}

pub fn read_tokens(filename: &str) -> TokenList {
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let tokens = serde_yaml::from_str(&yaml).unwrap();
    TokenList { tokens: tokens }
}

impl TokenList {
    pub fn get(&self, ticker: &types::Ticker) -> Option<&TokenDetail> {
        self.tokens.get(&ticker.symbol)
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BuySell {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

impl From<&types::AskBid> for BuySell {
    fn from(askbid: &types::AskBid) -> Self {
        match askbid {
            types::AskBid::Ask => BuySell::Buy,
            types::AskBid::Bid => BuySell::Sell,
        }
    }
}

impl Into<exchange::BuySell> for BuySell {
    fn into(self) -> exchange::BuySell {
        match self {
            BuySell::Buy => exchange::BuySell::Buy,
            BuySell::Sell => exchange::BuySell::Sell,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheetSign {
    #[serde(flatten)]
    sheet: OrderSheet,
    signature: String,
    address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    blockchain: String,
    contract_hash: String,
    order_type: String,
    pair: String,
    price: String,    // market-specified precision
    quantity: String, // integer unit quantity
    side: BuySell,
    timestamp: u128,
    use_native_tokens: bool,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderForm {
    blockchain: String,
    chain_id: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FillStatus {
    Pending,
    Confirming,
    Success,
    Canceling,
    Cancelled,
    Expired,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OrderStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "completed")]
    Completed,
}

impl OrderStatus {
    fn finto(&self) -> exchange::OrderState {
        match self {
            OrderStatus::Pending => exchange::OrderState::Pending,
            OrderStatus::Open => exchange::OrderState::Open,
            OrderStatus::Cancelled => exchange::OrderState::Cancelled,
            OrderStatus::Completed => exchange::OrderState::Filled,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Fill {
    id: String,
    offer_hash: Option<String>,
    offer_asset_id: String,
    want_asset_id: String,
    fill_amount: String,
    want_amount: String,
    filled_amount: Option<String>,
    fee_asset_id: String,
    fee_amount: String,
    maker_fee_amount: u128,
    price: String,
    txn: Option<String>,
    status: FillStatus,
    created_at: String,
    transaction_hash: Option<String>,
    burn_maker_fees: bool,
    contract_invocations: Option<String>,
}

trait Idable {
    fn id(&self) -> String;
}

impl Idable for Fill {
    fn id(&self) -> String {
        self.id.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillGroupTransactionScriptParamsArgs {
    fee_asset_id: String,
    fee_amount: String,
    filler: Option<String>,
    maker: Option<String>,
    nonce: u64,
    offer_asset_id: String,
    offer_amount: String,
    want_asset_id: String,
    want_amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MakeGroupTransactionScriptParamsArgs {
    fee_asset_id: String,
    fee_amount: u128,
    filler: Option<String>,
    maker: Option<String>,
    nonce: u64,
    offer_asset_id: String,
    offer_amount: String,
    want_asset_id: String,
    want_amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FillGroupTransactionScriptParams {
    args: FillGroupTransactionScriptParamsArgs,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MakeGroupTransactionScriptParams {
    args: MakeGroupTransactionScriptParamsArgs,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FillTaker {
    offer_hash: String,
    take_amount: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FillGroupTransaction {
    #[serde(rename = "chainId")]
    chain_id: String,
    hash: String,
    matches: Option<Vec<FillTaker>>,
    message: String,
    #[serde(rename = "offerHash")]
    offer_hash: Option<String>,
    script_params: FillGroupTransactionScriptParams,
    sha256: String,
    #[serde(rename = "typedPayload")]
    typed_payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MakeGroupTransaction {
    #[serde(rename = "chainId")]
    chain_id: String,
    hash: String,
    matches: Option<Vec<FillTaker>>,
    message: String,
    #[serde(rename = "offerHash")]
    offer_hash: Option<String>,
    script_params: MakeGroupTransactionScriptParams,
    sha256: String,
    #[serde(rename = "typedPayload")]
    typed_payload: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FillGroup {
    address: String,
    external: bool,
    fee_amount: String,
    fee_asset_id: String,
    fill_ids: Vec<String>,
    id: String,
    txn: Option<FillGroupTransaction>,
}

impl Display for FillGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "fillgroup: {}", self.address)
    }
}

impl Idable for FillGroup {
    fn id(&self) -> String {
        self.id.clone()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MakeGroup {
    id: String,
    offer_asset_id: String,
    offer_amount: String,
    want_asset_id: String,
    want_amount: String,
    price: String,
    status: FillStatus,
    fee_amount: u128,
    fee_asset_id: String,
    txn: Option<MakeGroupTransaction>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    id: String,
    blockchain: String,
    created_at: String,
    contract_hash: String,
    address: String,
    broadcast_cutoff_at: String,
    scheduled_cancellation_at: Option<String>,
    order_status: OrderStatus,
    side: BuySell,
    price: String,
    quantity: String,
    pair: String,
    fills: Vec<Fill>,
    fill_groups: Vec<FillGroup>,
    makes: Vec<MakeGroup>,
}

impl Order {
    fn into_exg(self, base_token: &TokenDetail, quote_token: &TokenDetail) -> exchange::Order {
        let date = chrono::DateTime::parse_from_str(self.created_at.as_str(), "%+").unwrap();
        exchange::Order {
            id: self.id,
            side: self.side.into(),
            state: self.order_status.finto(),
            market: self.pair,
            base_qty: units_to_amount(&self.quantity, base_token),
            quote: self.price.parse::<f64>().unwrap(),
            create_date: date.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseError {
    error: String,
    #[serde(default)]
    error_message: String,
    #[serde(default)]
    error_code: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceResponse {
    confirming: HashMap<String, Vec<BalanceConfirming>>,
    confirmed: HashMap<String, String>,
    locked: HashMap<String, String>,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct BalanceConfirming {
    id: String,
    event_type: String,
    asset_id: String,
    amount: String,
    transaction_hash: String,
    created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureSheet {
    makes: HashMap<String, String>,
    fills: HashMap<String, String>,
    fill_groups: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureBody {
    signatures: SignatureSheet,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawlRequest {
    amount: String,
    asset_id: String,
    blockchain: String,
    contract_hash: String,
    timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawlRequestSigned {
    #[serde(flatten)]
    withdrawl_request: WithdrawlRequest,
    signature: String,
    address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawalTransaction {
    sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawlResponse {
    id: String,
    transaction: WithdrawalTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawalExecute {
    id: String,
    timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawalExecuteSigned {
    #[serde(flatten)]
    withdrawal_execute: WithdrawalExecute,
    signature: String,
}

pub struct Switcheo {
    tokens: TokenList,
    pairs: PairList,
    settings: config::ExchangeSettings,
}

impl Switcheo {
    fn settings(&self) -> &config::ExchangeSettings {
        &self.settings
    }

    pub fn new(settings: config::ExchangeSettings) -> Switcheo {
        let tokens = read_tokens("notes/switcheo-tokens.json");
        let pairs = read_pairs("notes/switcheo-pairs.json");
        println!(
            "switcheo loaded {} tokens and {} pairs",
            tokens.len(),
            pairs.len()
        );
        Switcheo {
            tokens: tokens,
            pairs: pairs,
            settings: settings,
        }
    }

    pub fn wait_on_order(
        &self,
        order_id: &str,
        first_status: exchange::OrderState,
    ) -> exchange::OrderState {
        let mut status: exchange::OrderState = first_status;
        loop {
            let refresh = match status {
                exchange::OrderState::Pending | exchange::OrderState::Open => true,
                _ => false,
            };
            if refresh {
                println!("waiting");
                time::sleep(1000);
                println!("checking again {} {:?}", order_id, status);
                status = self.order_status(order_id);
            } else {
                println!("status good! {} {:?}", order_id, status);
                break;
            }
        }
        status
    }

    pub fn wait_on_ids(&self, ids: Vec<String>) {
        let cache = HashMap::<String, cell::Cell<Option<exchange::OrderState>>>::new();
        let mut stats = ids.into_iter().fold(cache, |mut m, i| {
            let c = cell::Cell::new(Option::<exchange::OrderState>::None);
            m.insert(i, c);
            m
        });
        loop {
            stats.iter_mut().for_each(|(id, cell)| {
                let refresh = match cell.get_mut() {
                    None => true,
                    Some(os) => match os {
                        exchange::OrderState::Pending | exchange::OrderState::Open => true,
                        _ => false,
                    },
                };
                if refresh {
                    let status = self.order_status(&id);
                    cell.set(Some(status));
                    //println!("waiting {} = {:?}", id, status);
                }
            });
            if stats
                .iter_mut()
                .any(|(id, status_opt)| match status_opt.get_mut() {
                    None => true,
                    Some(os) => match os {
                        exchange::OrderState::Pending | exchange::OrderState::Open => true,
                        _ => false,
                    },
                })
            {
                time::sleep(1000);
            } else {
                break;
            }
        }
    }
}

impl exchange::Api for Switcheo {
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
            "={:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );

        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        let market_pair = make_market_pair(market);
        let now_millis = time::now_millis();
        let base_token_detail = self.tokens.get(&market.base).unwrap();
        let quote_token_detail = self.tokens.get(&market.quote).unwrap();
        let pair = self.pairs.get(&market_pair).unwrap();

        let sheet = OrderSheet {
            blockchain: "eth".to_string(),
            contract_hash: exchange.contract_address.to_string(),
            order_type: "limit".to_string(),
            pair: market_pair,
            price: float_precision_string(offer.quote, pair.precision),
            quantity: amount_to_units(
                offer.base_qty,
                base_token_detail.precision,
                base_token_detail,
            ),
            side: askbid.into(),
            timestamp: now_millis,
            use_native_tokens: false,
        };
        let sign_json = serde_json::to_string(&sheet).unwrap();
        let signature = eth::ethsign(&sign_json, &secret_key);
        let address = format!("0x{}", eth::privkey_to_addr(privkey));
        println!("{:#?}", sheet);
        let sheet_sign = OrderSheetSign {
            address: address,
            sheet: sheet,
            signature: signature,
        };

        let url = format!("{}/orders", exchange.api_url.as_str());
        println!("switcheo build {}", url);
        println!("{}", serde_json::to_string(&sheet_sign.sheet).unwrap());
        let client = reqwest::blocking::Client::new();
        let resp = client.post(url.as_str()).json(&sheet_sign).send().unwrap();
        let status = resp.status();
        println!("switcheo build result {:#?} {}", status, resp.url());
        if status.is_success() {
            let json = resp.text().unwrap();
            //println!("{}", json);
            let order = serde_json::from_str::<Order>(&json).unwrap();
            for fill in &order.fills {
                println!(
                    "{}",
                    fill_display(fill, base_token_detail, quote_token_detail)
                );
            }
            for make in &order.makes {
                println!(
                    "{}",
                    makegroup_display(make, base_token_detail, quote_token_detail)
                );
            }
            Ok(exchange::OrderSheet::Switcheo(order))
        } else {
            let build_err = resp.json::<ResponseError>().unwrap();
            let order_error = exchange::OrderError {
                msg: build_err.error,
                code: build_err.error_code as i32,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    fn submit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        if let exchange::OrderSheet::Switcheo(order) = sheet {
            let url = format!(
                "{}/orders/{}/broadcast",
                exchange.api_url.as_str(),
                order.id
            );
            println!("{}", url);
            let makes = makes_sigs(&order.makes, &secret_key);
            let fill_groups = fillgroup_sigs(&order.fill_groups, &secret_key);
            let sig_sheet = SignatureBody {
                signatures: SignatureSheet {
                    fill_groups: fill_groups,
                    makes: makes,
                    fills: HashMap::new(),
                },
            };
            let client = reqwest::blocking::Client::new();
            let json = serde_json::to_string(&sig_sheet).unwrap();
            println!("switcheo submit {}", json);
            let resp = client.post(url.as_str()).json(&sig_sheet).send().unwrap();
            let status = resp.status();
            println!("{} {:?}", resp.status(), resp.text());
            if status.is_success() {
                // wait for success
                //let order_ids = gather_ids(sig_sheet.signatures);
                let id_status = self.wait_on_order(&order.id, order.order_status.finto());
                match id_status {
                    exchange::OrderState::Filled => {
                        println!("order filled!");
                        let (base_symbol, quote_symbol) = split_market_pair(&order.pair);
                        let token_symbol = match order.side {
                            BuySell::Buy => base_symbol,
                            BuySell::Sell => quote_symbol,
                        };
                        let token = types::Ticker {
                            symbol: token_symbol,
                        };
                        let token_detail = self.tokens.get(&token).unwrap();
                        let base_qty = units_to_amount(&order.quantity, token_detail);
                        let qty = match order.side {
                            BuySell::Buy => base_qty,
                            BuySell::Sell => {
                                let price = order.price.parse::<f64>().unwrap();
                                base_qty * price
                            }
                        };
                        self.withdrawl(privkey, exchange, qty, token);
                    }
                    exchange::OrderState::Cancelled => {
                        println!("order cancelled!");
                    }
                    _ => {
                        println!("order whatnow!");
                    }
                }
                Ok(())
            } else {
                Ok(())
            }
        } else {
            Ok(())
        }
    }

    fn balances<'a>(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> HashMap<String, f64> {
        let url = format!(
            "{}/balances?addresses=0x{}&contract_hashes={}",
            exchange.api_url.as_str(),
            public_addr,
            exchange.contract_address
        );
        let client = reqwest::blocking::Client::new();
        let resp = client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        let json = resp.text().unwrap();
        let balances = serde_json::from_str::<BalanceResponse>(&json).unwrap();
        if balances.confirming.len() > 0 {
            println!(
                "warning: switcheo confirming balances {:?}",
                balances.confirming.keys()
            )
        }
        balances
            .confirmed
            .iter()
            .map(|(symbol, units)| {
                match self.tokens.get(&types::Ticker {
                    symbol: symbol.to_string(),
                }) {
                    Some(token) => {
                        let f_bal = units_to_amount(units, token);
                        (symbol.clone(), f_bal)
                    }
                    None => (format!("conversion-err {} {}", symbol, units), 0.0),
                }
            })
            .collect()
    }

    fn withdrawl(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: types::Ticker,
    ) {
        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        let token_detail = self.tokens.get(&token).unwrap();
        let units = amount_to_units(amount, token_detail.precision, token_detail);
        let withdrawl_request = WithdrawlRequest {
            blockchain: "eth".to_string(),
            asset_id: token_detail.hash.clone(),
            amount: units,
            timestamp: time::now_millis(),
            contract_hash: exchange.contract_address.clone(),
        };
        let sign_json = serde_json::to_string(&withdrawl_request).unwrap();
        let signature = eth::ethsign(&sign_json, &secret_key);
        let address = format!("0x{}", eth::privkey_to_addr(privkey));
        let withdrawl_request_sign = WithdrawlRequestSigned {
            withdrawl_request: withdrawl_request,
            address: address,
            signature: signature,
        };
        let url = format!("{}/withdrawals", exchange.api_url.as_str());
        let client = reqwest::blocking::Client::new();
        let resp = client
            .post(url.as_str())
            .json(&withdrawl_request_sign)
            .send()
            .unwrap();
        let status = resp.status();
        println!("switcheo withdrawal request {:#?} {}", status, resp.url());
        let json = resp.text().unwrap();
        println!("{}", json);
        if status.is_success() {
            let resp = serde_json::from_str::<WithdrawlResponse>(&json).unwrap();
            let withdrawal_execute = WithdrawalExecute {
                id: resp.id,
                timestamp: time::now_millis(),
            };
            let signature = sha_hex_sign(&resp.transaction.sha256, &secret_key);
            let withdrawal_execute_signed = WithdrawalExecuteSigned {
                withdrawal_execute: withdrawal_execute,
                signature: signature,
            };
            let url = format!(
                "{}/withdrawals/{}/broadcast",
                exchange.api_url.as_str(),
                withdrawal_execute_signed.withdrawal_execute.id
            );
            let resp = client
                .post(url.as_str())
                .json(&withdrawal_execute_signed)
                .send()
                .unwrap();
            let status = resp.status();
            println!("switcheo withdrawal execute {:#?} {}", status, resp.url());
            let json = resp.text().unwrap();
            println!("{}", json);
        } else {
            let resp_err = serde_json::from_str::<ResponseError>(&json).unwrap();
            let order_error = exchange::OrderError {
                msg: resp_err.error,
                code: resp_err.error_code as i32,
            };
            println!("ERR: {}", order_error);
            //Err(Box::new(order_error));
        }
    }

    fn order_status(&self, order_id: &str) -> exchange::OrderState {
        let url = format!("{}/orders/{}", self.settings().api_url.as_str(), order_id);
        println!("{}", url);
        let client = reqwest::blocking::Client::new();
        let resp = client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        if status.is_success() {
            let order = resp.json::<Order>().unwrap();
            order.order_status.finto()
        } else {
            exchange::OrderState::Cancelled
        }
    }

    fn open_orders(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<exchange::Order> {
        let my_addr = eth::privkey_to_addr(private_key);
        let url = format!(
            "{}/orders?address=0x{}&contract_hashes={}",
            exchange.api_url.as_str(),
            my_addr,
            exchange.contract_address
        );
        println!("{}", url);
        let client = reqwest::blocking::Client::new();
        let resp = client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        if status.is_success() {
            let orders = resp.json::<Vec<Order>>().unwrap();
            let shortlist = Vec::<exchange::Order>::new();
            orders.into_iter().fold(shortlist, |mut m, o| {
                let (base_name, quote_name) = split_market_pair(&o.pair);
                match self.tokens.get(&types::Ticker {
                    symbol: base_name.to_string(),
                }) {
                    Some(base_token) => {
                        match self.tokens.get(&types::Ticker {
                            symbol: quote_name.to_string(),
                        }) {
                            Some(quote_token) => m.push(o.into_exg(base_token, quote_token)),
                            None => (),
                        }
                    }
                    None => (),
                }
                m
            })
        } else {
            let build_err = resp.json::<ResponseError>().unwrap();
            println!("{:?}", build_err);
            vec![] // bad
        }
    }
}

pub fn gather_ids(sigsheet: SignatureSheet) -> Vec<String> {
    let mut ids = vec![];
    for pair in sigsheet.fill_groups {
        ids.push(pair.0);
    }
    for pair in sigsheet.makes {
        ids.push(pair.0);
    }
    ids
}

pub fn float_precision_string(num: f64, precision: i32) -> String {
    // if its dumb and it works it aint dumb
    match precision {
        2 => format!("{:0.2}", num),
        3 => format!("{:0.3}", num),
        4 => format!("{:0.4}", num),
        5 => format!("{:0.5}", num),
        6 => format!("{:0.6}", num),
        7 => format!("{:0.7}", num),
        8 => format!("{:0.2}", num),
        _ => "float_precison_string err".to_string(),
    }
}

// todo: use Itable trait and dyn box sized voodoo
pub fn fillgroup_sigs(fgs: &Vec<FillGroup>, key: &SecretKey) -> HashMap<String, String> {
    fgs.iter().fold(HashMap::new(), |mut memo, fillg| {
        let sigsha = sha_hex_sign(&fillg.txn.as_ref().unwrap().sha256, key);
        memo.insert(fillg.id.clone(), sigsha);
        memo
    })
}

pub fn makes_sigs(fgs: &Vec<MakeGroup>, key: &SecretKey) -> HashMap<String, String> {
    fgs.iter().fold(HashMap::new(), |mut memo, fillg| {
        let sigsha = sha_hex_sign(&fillg.txn.as_ref().unwrap().sha256, key);
        memo.insert(fillg.id.clone(), sigsha);
        memo
    })
}
pub fn sha_hex_sign(sha_hex: &str, key: &SecretKey) -> String {
    let sha_bytes = hex::decode(&sha_hex[2..]).unwrap();
    let sig_bytes = eth::sign_bytes(&sha_bytes, key);
    format!("0x{}", hex::encode(sig_bytes.to_vec()))
}

pub fn make_market_pair(market: &exchange::Market) -> String {
    format!("{}_{}", market.base.symbol, market.quote.symbol)
}

pub fn split_market_pair(pair: &str) -> (String, String) {
    let parts: Vec<&str> = pair.split("_").collect();
    (parts[0].to_string(), parts[1].to_string())
}

pub fn amount_to_units(amount: f64, precision: i32, token: &TokenDetail) -> String {
    let qty_int = exchange::quantity_in_base_units(amount, precision, token.decimals);
    let qty_str = qty_int.to_str_radix(10);
    println!(
        "{}^{} {}^{} => \"{}\"",
        amount, precision, token.symbol, token.decimals, qty_str
    );
    qty_str
}

pub fn units_to_amount(units: &str, token: &TokenDetail) -> f64 {
    //thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: ParseIntError { kind: InvalidDigit }', src/exchanges/switcheo.rs:775:16
    let unts = units.parse::<u128>().unwrap();
    let power = 10_u128.pow(token.decimals as u32);
    unts as f64 / power as f64
}

pub fn fill_display(fill: &Fill, base_token: &TokenDetail, quote_token: &TokenDetail) -> String {
    let qty = units_to_amount(&fill.fill_amount, base_token);
    let cost = units_to_amount(&fill.want_amount, quote_token);
    format!("fill: {}@{} cost:{}", qty, fill.price, cost)
}

pub fn makegroup_display(
    mg: &MakeGroup,
    base_token: &TokenDetail,
    quote_token: &TokenDetail,
) -> String {
    let qty = units_to_amount(&mg.offer_amount, base_token);
    let cost = units_to_amount(&mg.want_amount, quote_token);
    format!("makegroup: {}@{} cost:{}", qty, mg.price, cost)
}

#[cfg(test)]
mod tests {
    use super::*;

    static privkey: &str = "98c193239bff9eb53a83e708b63b9c08d6e47900b775402aca2acc3daad06f24";

    #[test]
    fn test_order_sign() {
        let json = "{\"apple\":\"Z\",\"blockchain\":\"eth\",\"timestamp\":1529380859}";
        println!("privkey {} {}", &privkey, &json);
        let privkey_bytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privkey_bytes).unwrap();
        let signature = eth::ethsign(&json.to_string(), &secret_key);
        println!("json sign signature {}", signature);
        let good_sig = "0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d86c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e32911c";
        assert_eq!(signature, good_sig)
    }

    #[test]
    fn test_amount_to_units() {
        let token = TokenDetail {
            symbol: "BAT".to_string(),
            name: "BAT".to_string(),
            r#type: "wut".to_string(),
            hash: "abc".to_string(),
            decimals: 18,
            transfer_decimals: 18,
            precision: 2,
            minimum_quantity: "0".to_string(),
            trading_active: true,
            is_stablecoin: false,
            stablecoin_type: None,
        };
        let units = amount_to_units(2.3, 2, &token);
        assert_eq!(units, "2300000000000000000") // float sigma fun
    }

    #[test]
    fn test_units_to_amount() {
        let token = TokenDetail {
            symbol: "BAT".to_string(),
            name: "BAT".to_string(),
            r#type: "wut".to_string(),
            hash: "abc".to_string(),
            decimals: 8,
            transfer_decimals: 8,
            precision: 2,
            minimum_quantity: "0".to_string(),
            trading_active: true,
            is_stablecoin: false,
            stablecoin_type: None,
        };
        let amt = units_to_amount("123456789", &token);
        assert_eq!(amt, 1.23456789)
    }

    #[test]
    fn test_fillgroup_sigs() {
        let sha256 = "b64c9ca323f29f9de97212bc108361aa9d28bc2feccafd9bd6caf5e40a4cc7e7";
        let sha_bytes = hex::decode(sha256).unwrap();
        let privkey_bytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privkey_bytes).unwrap();
        let sig_bytes = eth::sign_bytes(&sha_bytes, &secret_key);
        let sigsha = format!("0x{}", hex::encode(sig_bytes.to_vec()));
        assert_eq!(sigsha, "0xee4bcd2862de81ce2a4d2ef8a7739844896c4d3098c9e6dcee0ba36efc62aa5a629e6e5ae004f2acd14e1c9d9f6d25a8b2dbb45311a205669706ad19b97e94e01b");
    }
}

/*
>  web3.eth.accounts.sign('{"apple":"Z","blockchain":"eth","timestamp":1529380859}',
                  '0x98c193239bff9eb53a83e708b63b9c08d6e47900b775402aca2acc3daad06f24')
{ message: '{"apple":"Z","blockchain":"eth","timestamp":1529380859}',
  messageHash: '0xd912c2d8ddef5f07bfa807be8ddb4d579ab978f52ab1176deea8b260f146ea21',
  v: '0x1c',
  r: '0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d8',
  s: '0x6c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e3291',
  signature: '0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d86c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e32911c' }
*/
