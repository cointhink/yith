use redis::{Commands, RedisError};

mod config;
mod order;

fn main() {
    let filename = "config.yaml";
    let config = config::load(filename);
    println!("Yith. {} loaded.", filename);
    app(config).unwrap();
}

fn app(config: config::Config) -> Result<u32, RedisError> {
    let mut client = rdsetup(&config.redis_url)?;
    let inplay_exists = client.exists("inplay")?;
    let arb_id = match inplay_exists {
        true => {
            println!("active order found!");
            rd_inplay(&mut client)
        }
        false => {
            println!("no active order. waiting for order.");
            rd_next_order(config)
        }
    }?;

    let order: order::Order = rd_order(&mut client, arb_id)?;
    println!("{:#?}", order);
    Ok(0)
}

fn rd_order(client: &mut redis::Connection, arb_id: String) -> Result<order::Order, RedisError> {
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let order: order::Order = serde_yaml::from_str(&json).unwrap();
    Ok(order)
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
    let mut pubclient = rdsetup(&config.redis_url)?;
    let mut ps = rdsub(&mut pubclient);

    let msg = ps.get_message()?;
    let new_id: String = msg.get_payload()?;
    println!("new Order {:#?}", new_id);
    Ok(new_id)
}

fn rd_inplay(client: &mut redis::Connection) -> Result<String, redis::RedisError> {
    let inplay: String = client.get("inplay")?;
    Ok(inplay)
}
