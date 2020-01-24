use std::fs;
use std::process;

use redis::{Commands, RedisError};
use yaml_rust::{Yaml, YamlLoader};

fn main() {
    println!("Yith");
    let config = configload();
    app(config).unwrap();
}

fn app(config: Vec<Yaml>) -> Result<u32, RedisError> {
    let redis_url = "redis://127.0.0.1/";
    let mut pubclient = rdsetup(redis_url)?;
    let mut ps = rdsub(&mut pubclient);

    let msg = ps.get_message()?;
    let arb_id: String = msg.get_payload()?;
    println!("redis {}", arb_id);

    let mut client = rdsetup(redis_url)?;
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let data = json::parse(&json).unwrap();
    println!("{} json {} bytes", hkey, data.len());
    Ok(0)
}

fn configload() -> Vec<Yaml> {
    let filename = "config.yaml";
    let yamlOk = fs::read_to_string(filename);
    let yaml = match yamlOk {
        Ok(str) => str,
        Err(error) => {
            panic!("{:?}", error)
        },
    };
    YamlLoader::load_from_str(&yaml).unwrap()
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
