use redis::Commands;

fn main() {
    println!("Yith");
    let redis_url = "redis://127.0.0.1/";
    let mut pubclient = rdsetup(redis_url).unwrap();
    let mut ps = rdsub(&mut pubclient);

    let msg = ps.get_message().unwrap();
    let arb_id: String = msg.get_payload().unwrap();
    println!("redis {}", arb_id);

    let mut client = rdsetup(redis_url).unwrap();
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json").unwrap();
    let data = json::parse(&json).unwrap();
    println!("{} json {} bytes", hkey, data.len());
}

fn rdsetup(url: &str) -> Result<redis::Connection, redis::RedisError> {
    let client = redis::Client::open(url)?;
    let con = client.get_connection()?;
    Ok(con)
}

fn rdsub<'a>(c: &'a mut redis::Connection) -> redis::PubSub<'a> {
    let mut ps = c.as_pubsub();
    let _ = ps.subscribe("orders");
    ps
}
