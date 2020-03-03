use crate::exchange;
use crate::types;
use crate::config;

pub struct Switcheo {}

impl exchange::Api for Switcheo {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeApi,
        market: &types::Market,
        offer: &types::Offer,
        proxy: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!(
            "Switcheo build {:#?} {} {}@{}",
            askbid, market, offer.base_qty, offer.quote
        );
    	Ok(())
    }
}