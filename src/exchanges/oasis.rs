use crate::config;
use crate::eth;
use crate::exchange;
use crate::types;
use std::collections;
use std::error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderSheet {
    address: String,
    token_buy: String,
    amount_buy: String,
    token_sell: String,
    amount_sell: String,
}

pub struct Oasis {}

impl exchange::Api for Oasis {
    fn build(
        &self,
        privkey: &str,
        askbid: &types::AskBid,
        exchange: &config::ExchangeSettings,
        market: &exchange::Market,
        offer: &types::Offer,
    ) -> Result<exchange::OrderSheet, Box<dyn error::Error>> {
    	let order_sheet = OrderSheet { 
    		address: "".to_string(),
    		token_buy: "".to_string(),
    		amount_buy: "".to_string(),
    		token_sell: "".to_string(),
    		amount_sell: "".to_string(),
    	};
        Ok(exchange::OrderSheet::Oasis(order_sheet))
    }

    fn submit(
        &self,
        private_key: &str,
        exchange: &config::ExchangeSettings,
        sheet: exchange::OrderSheet,
    ) -> Result<(), Box<dyn error::Error>> {
        Ok(())
    }

    fn balances<'a>(
        &self,
        privkey: &str,
        exchange: &config::ExchangeSettings,
    ) -> exchange::BalanceList {
        collections::HashMap::new()
    }
}
