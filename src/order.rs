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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Market {
    source: Source,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Source {
    name: String,
    url: String,
}
