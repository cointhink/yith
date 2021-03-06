use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub date: String,
    pub pair: Pair,
    pub ask_books: Books,
    pub bid_books: Books,
    pub cost: f64,
    pub trade_profit: f64,
    pub profit: f64,
    pub fee_network: f64,
    pub quote_usd: f64,
    pub network_usd: f64,
}

impl Order {
    pub fn from_file(arb_id: String) -> Order {
        let filename = format!("{}/order", arb_id);
        let json = std::fs::read_to_string(filename).expect("order json file bad");
        let order: Order = serde_yaml::from_str(&json).unwrap();
        order
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair {
    #[serde(rename = "Field0")]
    pub base: String, //base
    #[serde(rename = "Field1")]
    pub quote: String, //quote
}

impl fmt::Display for Pair {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.base, self.quote)
    }
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
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

impl fmt::Display for AskBid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let word = match self {
            AskBid::Ask => "ASK",
            AskBid::Bid => "BID",
        };
        write!(f, "{}", word)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Books {
    pub askbid: AskBid,
    pub books: Vec<Book>,
}

impl Books {
    #[allow(dead_code)]
    pub fn cost_total(&self) -> f64 {
        self.books
            .iter()
            .map(|b: &Book| b.cost_total(self.askbid))
            .sum()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Book {
    pub market: Market,
    pub offers: Vec<Offer>,
}

impl Book {
    #[allow(dead_code)]
    pub fn cost_total(&self, askbid: AskBid) -> f64 {
        self.offers.iter().map(|o| o.cost(askbid)).sum()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Market {
    pub source: Source,
    pub base: Ticker,
    pub base_contract: String,
    pub quote: Ticker,
    pub quote_contract: String,
    pub swapped: bool,
    pub quantity_decimals: f64,
    pub price_decimals: f64,
    pub min_order_size: String,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Ticker {
    pub symbol: String,
}

impl fmt::Display for Ticker {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.symbol)
    }
}

impl From<&str> for Ticker {
    fn from(symbol: &str) -> Self {
        Ticker {
            symbol: symbol.to_uppercase(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Offer {
    pub base_qty: f64,
    pub quote: f64,
}

impl fmt::Display for Offer {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:0.5}@{:0.5}", self.base_qty, self.quote)
    }
}

impl Offer {
    pub fn swap(&self) -> (f64, f64) {
        let s_qty = self.base_qty * self.quote;
        let s_quote = 1.0 / self.quote;
        (s_qty, s_quote)
    }

    pub fn cost(&self, askbid: AskBid) -> f64 {
        match askbid {
            AskBid::Ask => self.base_qty * self.quote,
            AskBid::Bid => self.base_qty,
        }
    }
}
