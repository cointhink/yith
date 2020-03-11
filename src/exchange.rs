use crate::config;
use crate::exchanges;
use crate::types;
use std::error;
use std::fmt;

pub enum OrderSheet {
    Ddex3(exchanges::ddex3::OrderSheet),
    Ddex4(exchanges::ddex4::OrderSheet),
    Zeroex(exchanges::zeroex::OrderSheet),
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

pub trait Api {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeApi,
        market: &types::Market,
        offer: &types::Offer,
        proxy: &str,
    ) -> Result<OrderSheet, Box<dyn error::Error>>;

    fn submit(&self, sheet: OrderSheet) -> Result<(), Box<dyn error::Error>>;
    fn balance<'a>(&self, public_addr: &str, contract: &str) -> f64 { 0.0 }
}

pub trait Balance {
    fn balance<'a>(&self, public_addr: &str, contract: &str) -> f64;
}
