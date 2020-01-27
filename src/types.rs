use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    id: String,
    pair: Pair,
    cost: f64,
    profit: f64,
    ask_books: Books,
    bid_books: Books,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair {
    Field0: String, //base
    Field1: String, //quote
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Books {
    askbid: String,
    books: Vec<Book>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Book {
    market: Market,
    offers: Vec<Offer>
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Market {
    source: Source,
    base: Ticker,
    quote: Ticker,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    name: String,
    url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Ticker {
    symbol: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Offer {
    base_qty: f64,
    quote: f64,
}