use crate::time;
use crate::types;
pub use redis::Commands; // re-export

pub type Connection = redis::Connection;
pub type Error = redis::RedisError;

pub struct Redis<'a> {
    pub url: &'a str,
    pub con: Connection,
}

impl Redis<'_> {
    pub fn new(url: &str) -> Redis {
        let client = redis::Client::open(url).unwrap();
        let con = client.get_connection().unwrap();
        Redis { url: url, con: con }
    }

    pub fn rd_next(&mut self) -> types::Order {
        let inplay_exists = self.rd_exists("inplay");
        let arb_id = match inplay_exists {
            true => {
                println!("active order found!");
                self.rd_inplay().unwrap()
            }
            false => {
                println!("no active order. waiting for order. {}", time::now_string());
                self.rd_next_order().unwrap()
            }
        };
        self.rd_order(arb_id).unwrap()
    }

    pub fn rd_next_order(&self) -> Result<String, Error> {
        let client = redis::Client::open(self.url)?;
        let mut con = client.get_connection()?;
        let mut ps = rdsub(&mut con, "orders");

        let msg = ps.get_message()?;
        let new_id: String = msg.get_payload()?;
        println!("new Order {:#?}", new_id);
        Ok(new_id)
    }

    pub fn rd_order(&mut self, arb_id: String) -> Result<types::Order, Error> {
        let hkey = format!("arb:{}", arb_id);
        let json: String = self.con.hget(&hkey, "json")?;
        let order: types::Order = serde_yaml::from_str(&json).unwrap();
        Ok(order)
    }

    pub fn rd_exists(&mut self, key: &str) -> bool {
        self.con.exists(key).unwrap()
    }

    pub fn rd_inplay(&mut self) -> Result<String, Error> {
        let inplay: String = self.con.get("inplay")?;
        Ok(inplay)
    }
}

pub fn rdsub<'a>(con: &'a mut Connection, channel: &str) -> redis::PubSub<'a> {
    let mut ps = con.as_pubsub();
    let _ = ps.subscribe(channel);
    ps
}
