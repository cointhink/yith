use serde::{Deserialize, Serialize};

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
    Field0: String, //base
    Field1: String, //quote
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Books {
    pub askbid: String,
    pub books: Vec<Book>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Book {
    pub market: Market,
    pub offers: Vec<Offer>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Market {
    pub source: Source,
    pub base: Ticker,
    pub quote: Ticker,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ticker {
    pub symbol: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Offer {
    pub base_qty: f64,
    pub quote: f64,
}