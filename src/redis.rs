use crate::config;
use crate::types;
pub use ::redis::{Commands}; // re-export

pub type Error = redis::RedisError;

pub fn rd_order(
    client: &mut redis::Connection,
    arb_id: String,
) -> Result<types::Order, Error> {
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let order: types::Order = serde_yaml::from_str(&json).unwrap();
    Ok(order)
}

pub fn rdsetup(url: &str) -> Result<redis::Connection, Error> {
    let client = redis::Client::open(url)?;
    let con = client.get_connection()?;
    Ok(con)
}

pub fn rdsub<'a>(con: &'a mut redis::Connection) -> redis::PubSub<'a> {
    let mut ps = con.as_pubsub();
    let _ = ps.subscribe("orders");
    ps
}

pub fn rd_next_order(config: &config::Config) -> Result<String, Error> {
    let mut pubclient = rdsetup(&config.redis_url)?;
    let mut ps = rdsub(&mut pubclient);

    let msg = ps.get_message()?;
    let new_id: String = msg.get_payload()?;
    println!("new Order {:#?}", new_id);
    Ok(new_id)
}

pub fn rd_inplay(client: &mut redis::Connection) -> Result<String, Error> {
    let inplay: String = client.get("inplay")?;
    Ok(inplay)
}
