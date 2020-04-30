use crate::config;
use crate::exchanges;
use crate::types;
use bigdecimal::BigDecimal;
use num_bigint::BigInt;
use num_traits::cast::FromPrimitive;
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
    Oasis(exchanges::oasis::OrderSheet),
    Placebo,
}

#[derive(Debug)]
pub struct ExchangeError {
    pub msg: String,
}

impl ExchangeError {
    pub fn build_box(msg: String) -> Box<dyn error::Error> {
        println!("{}", msg);
        Box::new(ExchangeError { msg: msg })
    }
}

impl error::Error for ExchangeError {
    fn description(&self) -> &str {
        &self.msg
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
    pub create_date: String,
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

pub type BalanceList = collections::HashMap<String, f64>;

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

#[allow(unused_variables)]
pub trait Api {
    fn setup(&mut self) {}

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

    fn market_minimum(
        &self,
        ticker: &types::Ticker,
        exchange: &config::ExchangeSettings,
    ) -> Option<f64> {
        println!(
            "warning {} has no market_minimum call ({})",
            exchange.name, ticker
        );
        None
    }

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

pub fn quantity_in_base_units(qty: f64, prec: i32, scale: i32) -> BigInt {
    let big_dec = BigDecimal::from_f64(qty)
        .unwrap()
        .with_scale(prec as i64) // truncates
        .with_scale(scale as i64);
    let (qty_int, _exp) = big_dec.into_bigint_and_exponent();
    qty_int
}

pub fn units_to_quantity(units: u64, scale: i32) -> f64 {
    let power = 10_u128.pow(scale as u32);
    units as f64 / power as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantity_in_base_units() {
        let unit_q = quantity_in_base_units(1.1234, 2, 18);
        assert_eq!(unit_q, 1120000000000000000_u64.into());
        let unit_q = quantity_in_base_units(100.1234, 2, 18);
        assert_eq!(unit_q, 100120000000000000000_u128.into());
        let unit_q = quantity_in_base_units(0.234, 8, 8);
        assert_eq!(unit_q, 23400000.into());
        let unit_q = quantity_in_base_units(2.3, 1, 2);
        assert_eq!(unit_q, 230.into());
    }
}
