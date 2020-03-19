use crate::config;
use crate::exchanges;
use crate::types;
use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OrderSheet {
    Ddex3(exchanges::ddex3::OrderSheet),
    Ddex4(exchanges::ddex4::OrderSheet),
    Zeroex(exchanges::zeroex::OrderForm),
    Switcheo(exchanges::switcheo::OrderSheetSign),
    Idex(exchanges::idex::OrderSheet),
}

#[derive(Debug)]
pub struct ExchangeError {}

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
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "display error here")
    }
}

#[derive(Debug)]
pub struct OrderError {
    pub msg: String,
    pub code: i32,
}

impl OrderError {
    pub fn new(msg: &str, code: i32) -> OrderError {
        OrderError {
            msg: msg.to_string(),
            code: code,
        }
    }
}

impl std::error::Error for OrderError {}

impl std::fmt::Display for OrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} [#{}]", self.msg, self.code)
    }
}

pub trait Api {
    fn setup(&mut self);
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &types::Market,
        offer: &types::Offer,
        proxy: Option<String>,
    ) -> Result<OrderSheet, Box<dyn error::Error>>;

    fn submit(
        &self,
        exchange: &config::ExchangeSettings,
        sheet: OrderSheet,
    ) -> Result<(), Box<dyn error::Error>>;
    fn balances<'a>(
        &self,
        public_addr: &str,
        ticker_symbol: Vec<&str>,
        contract: &str,
        exchange: &config::ExchangeSettings,
    ) -> Vec<(&str, f64)> {
        vec![]
    }
}
