
use redis::{Commands, RedisError};

mod order;
mod config;

fn main() {
    println!("Yith");
    let config = config::load();
    app(config).unwrap();
}

fn app(config: config::Config) -> Result<u32, RedisError> {
    let mut client = rdsetup(&config.redis)?;
    let inplay_exists = client.exists("inplay")?;
    let arb_id = match inplay_exists {
        true => rd_inplay(&mut client),
        false => rd_next_order(config),
    }?;
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let order: order::Order = serde_yaml::from_str(&json).unwrap();
    println!("{} {:#?}", hkey, order);
    Ok(0)
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

fn rd_next_order(config: config::Config) -> Result<String, redis::RedisError> {
    let mut pubclient = rdsetup(&config.redis)?;
    let mut ps = rdsub(&mut pubclient);

    println!("nothing active. waiting for order.");
    let msg = ps.get_message()?;
    let new_id: String = msg.get_payload()?;
    println!("new Order {:#?}", new_id);
    Ok(new_id)
}

fn rd_inplay(client: &mut redis::Connection) -> Result<String, redis::RedisError> {
    let inplay: String = client.get("inplay")?;
    println!("arb_id inplay {:#?}", inplay);
    Ok(inplay)
}
