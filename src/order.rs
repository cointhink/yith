use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    id: String,
    pair: Pair,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pair {
    base: String,
    quote: String,
}
