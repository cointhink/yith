use crate::exchange;

pub trait Ddex {
    fn make_market_id(&self, market: &exchange::Market) -> String {
        market.id("-")
    }
}
