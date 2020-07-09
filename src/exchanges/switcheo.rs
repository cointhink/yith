use crate::config;
use crate::eth;
use crate::exchange;
use crate::geth;
use crate::http;
use crate::log;
use crate::time;
use crate::types;
use secp256k1::SecretKey;
use serde::{Deserialize, Serialize};
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
    pub hash: String,
    pub decimals: i32,
    transfer_decimals: i32,
    precision: i32,
    minimum_quantity: String,
    trading_active: bool,
    is_stablecoin: bool,
    stablecoin_type: Option<String>,
}

#[derive(Debug)]
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
    fn into_exg(self, base_token: &TokenDetail, _quote_token: &TokenDetail) -> exchange::Order {
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
    transaction_hash: Option<String>,
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
pub struct TransferRequest {
    amount: String,
    asset_id: String,
    blockchain: String,
    contract_hash: String,
    timestamp: u128,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferRequestSigned {
    #[serde(flatten)]
    transfer_request: TransferRequest,
    signature: String,
    address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawalBuildResponse {
    id: String,
    transaction: WithdrawalTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WithdrawalTransaction {
    hash: String,
    message: String,
    sha256: String,
    chain_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositBuildResponse {
    id: String,
    transaction: DepositTransaction,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepositTransaction {
    from: String,
    value: String,
    to: String,
    data: String,
    gas: String,
    gas_price: String,
    #[serde(skip_deserializing)]
    chain_id: u32, // v2/withdrawals returns "1". v2/deposits returns 1
    nonce: String,
    sha256: String,
}

pub enum TransferDirection {
    Withdrawal,
    Deposit,
}

impl Into<ethereum_tx_sign::RawTransaction> for DepositTransaction {
    fn into(self) -> ethereum_tx_sign::RawTransaction {
        let mut sized_to = [0u8; 20];
        sized_to.copy_from_slice(&eth::dehex(&self.to)[..]);
        ethereum_tx_sign::RawTransaction {
            nonce: u64::from_str_radix(&self.nonce[2..], 16).unwrap().into(),
            to: Some(sized_to.into()),
            value: u64::from_str_radix(&self.value[2..], 16).unwrap().into(),
            gas_price: u64::from_str_radix(&self.gas_price[2..], 16)
                .unwrap()
                .into(),
            gas: u64::from_str_radix(&self.gas[2..], 16).unwrap().into(),
            data: eth::dehex(&self.data),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawalResponse {
    status: String,
    id: String,
    blockchain: String,
    transaction_hash: String,
    amount: String,
    asset_id: String,
    event_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositResponseOk {
    result: String,
    transaction_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TransferResponseErr {
    error: String,
    error_message: String,
    error_code: u32,
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DepositExecute {
    transaction_hash: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TimestampResponse {
    timestamp: u128,
}

pub struct Switcheo {
    geth: geth::Client,
    tokens: TokenList,
    pairs: PairList,
    pub settings: config::ExchangeSettings,
    client: http::LoggingClient,
}

impl Switcheo {
    pub fn new(settings: config::ExchangeSettings, geth: geth::Client) -> Switcheo {
        let tokens = read_tokens("notes/switcheo-tokens.json");
        let pairs = read_pairs("notes/switcheo-pairs.json");
        log::debug!(
            "switcheo loaded {} tokens and {} pairs",
            tokens.len(),
            pairs.len()
        );
        let client = reqwest::blocking::Client::new();
        let logging_client = http::LoggingClient::new(client);
        Switcheo {
            geth: geth,
            tokens: tokens,
            pairs: pairs,
            settings: settings,
            client: logging_client,
        }
    }

    pub fn transfer(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
        direction: TransferDirection,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        let token_detail = self.tokens.get(&token).unwrap();
        let units = amount_to_units(
            amount,
            token_detail.transfer_decimals - 1,
            token_detail.decimals,
        );
        let withdrawl_request = TransferRequest {
            blockchain: "eth".to_string(),
            asset_id: token_detail.hash.clone(),
            amount: units,
            timestamp: self.nonce(),
            contract_hash: exchange.contract_address.clone(),
        };
        let sign_json = serde_json::to_string(&withdrawl_request).unwrap();
        let signature = eth::ethsign(&sign_json, &secret_key);
        let address = format!("0x{}", eth::privkey_to_addr(privkey));
        let transfer_request_signed = TransferRequestSigned {
            transfer_request: withdrawl_request,
            address: address,
            signature: signature,
        };
        let api_word = match direction {
            TransferDirection::Withdrawal => "withdrawals",
            TransferDirection::Deposit => "deposits",
        };
        let url = format!("{}/{}", exchange.api_url.as_str(), api_word);
        let resp = self
            .client
            .post(url.as_str())
            .json(&transfer_request_signed)
            .send()
            .unwrap();
        let status = resp.status();
        println!("{} {}", resp.url(), status);
        let json = resp.text().unwrap();
        if status.is_success() {
            Ok(json.to_string())
        } else {
            let resp_err = serde_json::from_str::<ResponseError>(&json).unwrap();
            let order_error = exchange::OrderError {
                msg: resp_err.error,
                code: resp_err.error_code as i32,
            };
            println!("ERR: {}", order_error);
            Err(Box::new(order_error))
        }
    }

    pub fn nonce(&self) -> u128 {
        let url = format!("{}/timestamp", self.settings.api_url.as_str());
        let resp = self.client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        let timestamp = resp.json::<TimestampResponse>().unwrap().timestamp;
        timestamp
    }

    pub fn balances(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> BalanceResponse {
        let url = format!(
            "{}/balances?addresses=0x{}&contract_hashes={}",
            exchange.api_url.as_str(),
            public_addr,
            exchange.contract_address
        );
        let resp = self.client.get(url.as_str()).send().unwrap();
        let json = resp.text().unwrap();
        serde_json::from_str::<BalanceResponse>(&json).unwrap()
    }

    fn wait_confirming_balances(&self, public_addr: &str, exchange: &config::ExchangeSettings) {
        let mut repeat = true;
        while repeat {
            let balances = self.balances(public_addr, exchange);
            let balances_confirming = balances.confirming.len();
            repeat = if balances_confirming > 0 {
                println!(
                    "{} switcheo confirming balances. waiting...",
                    balances_confirming
                );
                time::sleep(5000);
                true
            } else {
                false
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
            quantity: amount_to_units(
                offer.base_qty * 0.99999, // f64 hack
                base_token_detail.precision,
                base_token_detail.decimals,
            ),
            price: float_to_string_precision(offer.quote, pair.precision),
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
        let resp = self
            .client
            .post(url.as_str())
            .json(&sheet_sign)
            .send()
            .unwrap();
        let status = resp.status();
        println!("switcheo build result {:#?} {}", status, resp.url());
        if status.is_success() {
            let json = resp.text().unwrap();
            //println!("{}", json);
            let order = serde_json::from_str::<Order>(&json).unwrap();
            println!("{} fills", &order.fills.len());
            for fill in &order.fills {
                println!(
                    "{}",
                    fill_display(fill, base_token_detail, quote_token_detail)
                );
            }
            println!("{} makegroups", &order.makes.len());
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
    ) -> Result<String, Box<dyn std::error::Error>> {
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
            let json = serde_json::to_string(&sig_sheet).unwrap();
            println!("switcheo submit {}", json);
            let resp = self
                .client
                .post(url.as_str())
                .json(&sig_sheet)
                .send()
                .unwrap();
            let status = resp.status();
            println!("{} {:?}", status, resp.text());
            if status.is_success() {
                Ok(order.id.clone())
            } else {
                Err(exchange::ExchangeError::build_box(format!(
                    "switcheo order post {}",
                    status
                )))
            }
        } else {
            Err(exchange::ExchangeError::build_box(format!(
                "wrong ordersheet type!"
            )))
        }
    }

    fn market_minimums(
        &self,
        market: &exchange::Market,
        _exchange: &config::ExchangeSettings,
    ) -> Option<(Option<f64>, Option<f64>)> {
        match self.tokens.get(&market.quote) {
            Some(base_token_detail) => {
                let min_cost =
                    units_to_amount(&base_token_detail.minimum_quantity, base_token_detail);
                Some((None, Some(min_cost)))
            }
            None => None,
        }
    }

    fn balances(
        &self,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> HashMap<String, f64> {
        let balances = self.balances(public_addr, exchange);
        if balances.confirming.len() > 0 {
            println!(
                "WARNING: switcheo confirming balances {:?}",
                balances.confirming
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

    fn transfer_status<'a>(
        &self,
        transfer_id: &str,
        public_addr: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceStatus {
        let balances = self.balances(public_addr, exchange);
        let record = balances.confirming.values().fold(None, |memo, ar| {
            match ar.iter().find(|r| r.id == transfer_id) {
                Some(item) => Some(item),
                None => memo,
            }
        });
        match record {
            Some(_tid) => exchange::BalanceStatus::InProgress,
            None => exchange::BalanceStatus::Complete,
        }
    }

    fn withdraw(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let privbytes = &hex::decode(privkey).unwrap();
        let secret_key = SecretKey::from_slice(privbytes).unwrap();
        let response = self.transfer(
            privkey,
            exchange,
            amount,
            token,
            TransferDirection::Withdrawal,
        );
        match response {
            Ok(json) => {
                let resp = serde_json::from_str::<WithdrawalBuildResponse>(&json).unwrap();
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
                let resp = self
                    .client
                    .post(url.as_str())
                    .json(&withdrawal_execute_signed)
                    .send()
                    .unwrap();
                let status = resp.status();
                let json = resp.text().unwrap();
                if status.is_success() {
                    let response = serde_json::from_str::<WithdrawalResponse>(&json).unwrap();
                    Ok(Some(response.id))
                } else {
                    println!("http err");
                    let err = serde_json::from_str::<TransferResponseErr>(&json).unwrap();
                    Err(exchange::ExchangeError::build_box(err.error_message))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn deposit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let response_opt =
            self.transfer(privkey, exchange, amount, token, TransferDirection::Deposit);
        match response_opt {
            Ok(json) => {
                let build_response = serde_json::from_str::<DepositBuildResponse>(&json).unwrap();
                let tx: ethereum_tx_sign::RawTransaction = build_response.transaction.into();
                let private_key = ethereum_types::H256::from_slice(&eth::dehex(privkey));
                let rlp_bytes = tx.sign(&private_key, &eth::ETH_CHAIN_MAINNET);
                let params = (eth::hex(&rlp_bytes),);
                let tx = self
                    .geth
                    .rpc_str("eth_sendRawTransaction", geth::ParamTypes::Single(params))?;
                println!("deposit approval {}", tx);
                let deposit_execute = DepositExecute {
                    transaction_hash: tx.clone(),
                };
                let url = format!(
                    "{}/deposits/{}/broadcast",
                    exchange.api_url.as_str(),
                    build_response.id
                );
                let resp = self
                    .client
                    .post(url.as_str())
                    .json(&deposit_execute)
                    .send()
                    .unwrap();
                let status = resp.status();
                let json = resp.text().unwrap();
                if status.is_success() {
                    let response = serde_json::from_str::<DepositResponseOk>(&json).unwrap();
                    let tx = eth::hex(&eth::hash_msg(&eth::dehex(&response.transaction_hash)));
                    println!("deposit tx {}", tx);
                    Ok(Some(build_response.id))
                } else {
                    let err = serde_json::from_str::<TransferResponseErr>(&json).unwrap();
                    Err(exchange::ExchangeError::build_box(err.error_message))
                }
            }
            Err(e) => Err(e),
        }
    }

    fn order_status(
        &self,
        order_id: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::OrderState {
        let url = format!("{}/orders/{}", self.settings.api_url.as_str(), order_id);
        println!("{}", url);
        let resp = self.client.get(url.as_str()).send().unwrap();
        let status = resp.status();
        if status.is_success() {
            let order = resp.json::<Order>().unwrap();
            match order.order_status.finto() {
                exchange::OrderState::Filled => {
                    // wait for confirming balances
                    println!("switcheo order_status shows Filled, waiting on confirming balances");
                    let config = config::CONFIG.get().unwrap();
                    let my_addr = eth::privkey_to_addr(&config.wallet_private_key);
                    self.wait_confirming_balances(&my_addr, exchange);
                    exchange::OrderState::Filled
                }
                e => e,
            }
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
        let resp = self.client.get(url.as_str()).send().unwrap();
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

#[allow(dead_code)]
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

pub fn amount_to_units(amount: f64, precision: i32, decimals: i32) -> String {
    let qty_int = exchange::quantity_in_base_units(amount, precision, decimals);
    let qty_str = qty_int.to_str_radix(10);
    qty_str
}

pub fn units_to_amount(units: &str, token: &TokenDetail) -> f64 {
    //thread 'main' panicked at 'called `Result::unwrap()` on an `Err` value: ParseIntError { kind: InvalidDigit }', src/exchanges/switcheo.rs:775:16
    let unts = units.parse::<u128>().unwrap();
    let power = 10_u128.pow(token.decimals as u32);
    unts as f64 / power as f64
}

pub fn float_to_string_precision(num: f64, precision: i32) -> String {
    let prec = precision as usize;
    let num_str = num.to_string();
    let parts: Vec<&str> = num_str.split(".").collect();
    let int = parts[0].parse::<i32>().unwrap();
    let mut frac = if parts.len() == 2 { &parts[1] } else { "" }.to_string();
    frac.truncate(prec);
    let padding = "0".repeat(prec - frac.len());
    format!("{}.{}{}", int, frac, padding)
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

    static PRIVKEY: &str = "98c193239bff9eb53a83e708b63b9c08d6e47900b775402aca2acc3daad06f24";

    #[test]
    fn test_order_sign() {
        let json = "{\"apple\":\"Z\",\"blockchain\":\"eth\",\"timestamp\":1529380859}";
        let privkey_bytes = &hex::decode(PRIVKEY).unwrap();
        let secret_key = SecretKey::from_slice(privkey_bytes).unwrap();
        let signature = eth::ethsign(&json.to_string(), &secret_key);
        let good_sig = "0xbcff177dba964027085b5653a5732a68677a66c581f9c85a18e1dc23892c72d86c0b65336e8a17637fd1fe1def7fa8cbac43bf9a8b98ad9c1e21d00e304e32911c";
        assert_eq!(signature, good_sig)
    }

    #[test]
    fn test_amount_to_units() {
        let units = amount_to_units(2.3, 2, 18);
        assert_eq!(units, "2300000000000000000"); // float sigma fun
        let units2 = amount_to_units(0.0001234, 6, 8);
        assert_eq!(units2, "12300"); // float sigma fun
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
        let privkey_bytes = &hex::decode(PRIVKEY).unwrap();
        let secret_key = SecretKey::from_slice(privkey_bytes).unwrap();
        let sig_bytes = eth::sign_bytes(&sha_bytes, &secret_key);
        let sigsha = format!("0x{}", hex::encode(sig_bytes.to_vec()));
        assert_eq!(sigsha, "0xee4bcd2862de81ce2a4d2ef8a7739844896c4d3098c9e6dcee0ba36efc62aa5a629e6e5ae004f2acd14e1c9d9f6d25a8b2dbb45311a205669706ad19b97e94e01b");
    }

    #[test]
    fn test_float_to_string_precision() {
        let float_str = float_to_string_precision(1.0, 1);
        assert_eq!(float_str, "1.0");
        let float_str = float_to_string_precision(1.0, 2);
        assert_eq!(float_str, "1.00");
        let float_str = float_to_string_precision(1.1, 2);
        assert_eq!(float_str, "1.10");
        let float_str = float_to_string_precision(1.12, 2);
        assert_eq!(float_str, "1.12");
        let float_str = float_to_string_precision(1.1234, 2);
        assert_eq!(float_str, "1.12");
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
