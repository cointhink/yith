use crate::config;
use crate::types;
pub use redis::Commands; // re-export

pub type Connection = redis::Connection;
pub type Error = redis::RedisError;

pub struct Redis<'a> {
    pub url: &'a str,
}

impl Redis<'_> {
    pub fn rd_next(&self, mut client: &mut Connection) -> types::Order {
        let inplay_exists = rd_exists(client, "inplay");
        let arb_id = match inplay_exists {
            true => {
                println!("active order found!");
                rd_inplay(&mut client).unwrap()
            }
            false => {
                println!("no active order. waiting for order.");
                let mut pubsub_client = rdsetup(self.url).unwrap();
                rd_next_order(&mut pubsub_client).unwrap()
            }
        };
        rd_order(&mut client, arb_id).unwrap()
    }
}

pub fn rd_order(client: &mut Connection, arb_id: String) -> Result<types::Order, Error> {
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let order: types::Order = serde_yaml::from_str(&json).unwrap();
    Ok(order)
}

pub fn rdsetup(url: &str) -> Result<Connection, Error> {
    let client = redis::Client::open(url)?;
    let con = client.get_connection()?;
    Ok(con)
}

pub fn rd_exists<'a>(client: &mut Connection, key: &str) -> bool {
    client.exists(key).unwrap()
}

pub fn rdsub<'a>(con: &'a mut Connection) -> redis::PubSub<'a> {
    let mut ps = con.as_pubsub();
    let _ = ps.subscribe("orders");
    ps
}

pub fn rd_next_order(client: &mut Connection) -> Result<String, Error> {
    let mut ps = rdsub(client);

    let msg = ps.get_message()?;
    let new_id: String = msg.get_payload()?;
    println!("new Order {:#?}", new_id);
    Ok(new_id)
}

pub fn rd_inplay(client: &mut Connection) -> Result<String, Error> {
    let inplay: String = client.get("inplay")?;
    Ok(inplay)
}
