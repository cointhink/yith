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

#[derive(Debug)]
pub enum TransferDirection {
    Deposit,
    Withdraw,
}

impl TransferDirection {
    pub fn read(direction: &str) -> Option<Self> {
        match direction {
            "withdraw" => Some(Self::Withdraw),
            "deposit" => Some(Self::Deposit),
            _ => None,
        }
    }
}

// impl From<&str> for TransferDirection {
//     fn from(direction: &str) -> Self {
//         match direction {
//             "withdrawal" => Self::Withdrawal,
//             "deposit" => Self::Deposit,
//         }
//     }
// }

impl std::fmt::Display for TransferDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let word = match self {
            Self::Deposit => "Deposit",
            Self::Withdraw => "Withdraw",
        };
        write!(f, "{}", word)
    }
}

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
pub enum BalanceStatus {
    Complete,
    InProgress,
    Error,
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
    ) -> Result<String, Box<dyn error::Error>>;

    fn market_minimums(
        &self,
        market: &Market,
        exchange: &config::ExchangeSettings,
    ) -> Option<(Option<f64>, Option<f64>)> {
        println!(
            "WARNING: {} has no market_minimum call ({})",
            exchange.name, market
        );
        None
    }

    fn balances<'a>(&self, privkey: &str, exchange: &config::ExchangeSettings) -> BalanceList {
        println!("WARNING: {} has no balances call", exchange.name);
        collections::HashMap::new()
    }

    fn transfer_status<'a>(
        &self,
        transfer_id: &str,
        privkey: &str,
        exchange: &config::ExchangeSettings,
    ) -> BalanceStatus {
        println!("WARNING: {} has no transfer_status call", exchange.name);
        BalanceStatus::InProgress
    }

    fn open_orders(&self, private_key: &str, exchange: &config::ExchangeSettings) -> Vec<Order> {
        println!("WARNING: {} has no open_orders call", exchange.name);
        vec![]
    }

    fn order_status(&self, order_id: &str, exchange: &config::ExchangeSettings) -> OrderState {
        println!("WARNING: no order_status call");
        OrderState::Open
    }

    fn withdraw(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) -> Result<Option<String>, Box<dyn error::Error>> {
        if exchange.has_balances {
            println!("WARNING: withdraw not implemented for {}", exchange.name);
            Ok(None)
        } else {
            Err(ExchangeError::build_box(
                "withdraw called on exchange with no balance support".to_string(),
            ))
        }
    }

    fn deposit(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
        amount: f64,
        token: &types::Ticker,
    ) -> Result<Option<String>, Box<dyn error::Error>> {
        if exchange.has_balances {
            println!("WARNING: deposit not implemented for {}", exchange.name);
            Ok(None)
        } else {
            Err(ExchangeError::build_box(
                "deposit called on exchange with no balance support".to_string(),
            ))
        }
    }
}

pub fn quantity_in_base_units(qty: f64, prec: i32, scale: i32) -> BigInt {
    let f64_str = qty.to_string();
    let parts = f64_str.split(".").collect::<Vec<_>>();
    let f64_int = parts[0].to_string();
    let mut f64_frac = parts[1].to_string();
    let f64_frac_max = 15 - f64_int.len(); // remaining digits define frac length
    let chop_size = std::cmp::min(f64_frac_max, prec as usize);
    f64_frac.truncate(chop_size);
    let padding = (scale as usize) - f64_frac.len();
    for n in 0..padding {
        f64_frac.push('0')
    }
    let int_str = format!("{}{}", f64_int, f64_frac);
    int_str.parse::<BigInt>().unwrap()
}

pub fn units_to_quantity(units: u128, scale: i32) -> f64 {
    let power = 10_u128.pow(scale as u32);
    units as f64 / power as f64
}

pub fn str_to_chopped_f64(number: &str) -> f64 {
    // parse the string, last digit is rounded (thats bad)
    let f64 = number.parse::<f64>().unwrap();
    chopped_f64(f64)
}

pub fn chopped_f64(number: f64) -> f64 {
    let parsed = number.to_string();
    // drop the last digit to always be lower
    parsed[..parsed.len() - 1].parse::<f64>().unwrap()
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
        let unit_q = quantity_in_base_units(2.38, 1, 2);
        assert_eq!(unit_q, 230.into());
        let unit_q = quantity_in_base_units(0.224177038020941482, 18, 18);
        assert_eq!(unit_q.to_string(), "224177038020940000");
        let unit_q = quantity_in_base_units(10.224177038020941482, 18, 18);
        assert_eq!(unit_q.to_string(), "10224177038020900000");
        let unit_q = quantity_in_base_units(123456789012345.1234567890123456789, 18, 18);
        assert_eq!(unit_q.to_string(), "123456789012345000000000000000000");
        let unit_q = quantity_in_base_units(3.764604555995115, 18, 18);
        assert_eq!(unit_q.to_string(), "3764604555995110000");
    }

    #[test]
    fn test_str_to_chopped_f64() {
        let unrepresentable = "0.221637009876543199";
        let nearest_float_chopped = 0.221637009876543_f64;
        let unit_q = str_to_chopped_f64(unrepresentable);
        assert_eq!(unit_q, nearest_float_chopped);

        let u2 = "4.721027191907876302";
        let n2 = 4.72102719190787;
        let q2 = str_to_chopped_f64(u2);
        assert_eq!(q2, n2);
    }
}
