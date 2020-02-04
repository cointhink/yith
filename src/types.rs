use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub pair: Pair,
    pub cost: f64,
    pub profit: f64,
    pub ask_books: Books,
    pub bid_books: Books,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair {
    #[serde(rename = "Field0")]
    pub base: String, //base
    #[serde(rename = "Field1")]
    pub quote: String, //quote
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AskBid {
    #[serde(rename = "ask")]
    Ask,
    #[serde(rename = "bid")]
    Bid,
}
impl AskBid {
    pub fn otherside(&self) -> AskBid {
        match self {
            AskBid::Ask => AskBid::Bid,
            AskBid::Bid => AskBid::Ask,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Books {
    pub askbid: AskBid,
    pub books: Vec<Book>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Book {
    pub market: Market,
    pub offers: Vec<Offer>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Market {
    pub source: Source,
    pub base: Ticker,
    pub quote: Ticker,
    pub swapped: bool,
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}:{}{}{}",
            self.source.name,
            self.base,
            if self.swapped { "<>" } else { "-" },
            self.quote
        )
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ticker {
    pub symbol: String,
}

impl fmt::Display for Ticker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Offer {
    pub base_qty: f64,
    pub quote: f64,
}

impl fmt::Display for Offer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}@{}", self.base_qty, self.quote)
    }
}

impl Offer {
    pub fn swap(&self) -> (f64, f64) {
        let s_qty = self.base_qty * self.quote;
        let s_quote = 1.0 / self.quote;
        (s_qty, s_quote)
    }
}
