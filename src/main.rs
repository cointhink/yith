use std::fs;

use redis::{Commands, RedisError};
use serde::{Deserialize, Serialize};

mod order;
use crate::order::Order;

fn main() {
    println!("Yith");
    let config = configload();
    app(config).unwrap();
}

fn app(config: Config) -> Result<u32, RedisError> {
    let mut client = rdsetup(&config.redis)?;
    let inplay_exists = client.exists("inplay")?;
    let arb_id: String;
    if inplay_exists {
        arb_id = client.get("inplay")?;
        println!("arb_id inplay {:#?}", arb_id);
    } else {
        let mut pubclient = rdsetup(&config.redis)?;
        let mut ps = rdsub(&mut pubclient);

        println!("nothing active. waiting for order.");
        let msg = ps.get_message()?;
        arb_id = msg.get_payload()?;
        println!("new Order {}", arb_id);

    }
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let order: Order = serde_yaml::from_str(&json).unwrap();
    println!("{} {:#?}", hkey, order);
    Ok(0)
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    redis: String,
}

fn configload() -> Config {
    let filename = "config.yaml";
    let file_ok = fs::read_to_string(filename);
    let yaml = file_ok.unwrap();
    let config: Config = serde_yaml::from_str(&yaml).unwrap();
    println!("{:#?}", config);
    config
}

fn rdsetup(url: &str) -> Result<redis::Connection, redis::RedisError> {
    let client = redis::Client::open(url)?;
    let con = client.get_connection()?;
    Ok(con)
}

fn rdsub<'a>(con: &'a mut redis::Connection) -> redis::PubSub<'a> {
    let mut ps = con.as_pubsub();
    let _ = ps.subscribe("orders");
    ps
}
