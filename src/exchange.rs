use crate::config;
use crate::types;

pub trait Api {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeApi,
        market: &types::Market,
        offer: &types::Offer,
        proxy: &str,
    ) -> Result<(), Box<dyn std::error::Error>>;
}
