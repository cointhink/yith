use crate::config;
use crate::exchanges;
use crate::types;
use serde::{Deserialize, Serialize};
use std::collections;
use std::error;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderSheet {
    Ddex3(exchanges::ddex3::OrderSheet),
    Ddex4(exchanges::ddex4::OrderSheet),
    Zeroex(exchanges::zeroex::OrderForm),
    Switcheo(exchanges::switcheo::Order),
    Idex(exchanges::idex::OrderSheet),
}

#[derive(Debug)]
pub struct ExchangeError {
    pub msg: String,
}

impl error::Error for ExchangeError {
    fn description(&self) -> &str {
        "it done goofed up"
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        // Generic error, underlying cause isn't tracked.
        None
    }
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

#[derive(Debug)]
pub enum BuySell {
    Buy,
    Sell,
}

impl fmt::Display for BuySell {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let word = match self {
            BuySell::Buy => "buy",
            BuySell::Sell => "sell",
        };
        write!(f, "{}", word)
    }
}

#[derive(Debug, PartialEq)]
pub enum OrderState {
    Pending,
    Open,
    Filled,
    Cancelled,
    Expired,
    Unfunded,
}

#[derive(Debug)]
pub struct Order {
    pub id: String,
    pub side: BuySell,
    pub state: OrderState,
    pub market: String,
    pub base_qty: f64,
    pub quote: f64,
    pub create_date: i64,
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {:0.5}@{:0.5}", self.side, self.base_qty, self.quote)
    }
}

#[derive(Debug)]
pub struct OrderError {
    pub msg: String,
    pub code: i32,
}

impl std::error::Error for OrderError {}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} [#{}]", self.msg, self.code)
    }
}

type BalanceList = collections::HashMap<String, f64>;

#[derive(Debug)]
pub struct Market {
    pub base: types::Ticker,
    pub base_contract: String,
    pub quote: types::Ticker,
    pub quote_contract: String,
    pub quantity_decimals: f64,
    pub price_decimals: f64,
    pub source_name: String,
}

impl Market {
    pub fn id(&self, seperator: &str) -> String {
        format!("{}{}{}", self.base.symbol, seperator, self.quote.symbol)
    }
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}/{}", self.base, self.quote)
    }
}

pub trait Api {
    fn setup(&mut self);
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &Market,
        offer: &types::Offer,
    ) -> Result<OrderSheet, Box<dyn error::Error>>;

    fn submit(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        sheet: OrderSheet,
    ) -> Result<(), Box<dyn error::Error>>;

    fn balances<'a>(&self, privkey: &str, exchange: &config::ExchangeSettings) -> BalanceList {
        collections::HashMap::new()
    }

    fn open_orders(&self, account: &str, exchange: &config::ExchangeSettings) -> Vec<Order> {
        println!("warning {} has no open_orders call", exchange.name);
        vec![]
    }

    fn order_status(&self, order_id: &str) -> OrderState {
        OrderState::Open
    }

    fn withdrawl(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: types::Ticker,
    ) {
    }
}
