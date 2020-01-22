use redis::Commands;

fn main() {
    println!("Yith");
    let mut pubclient = rdgo().unwrap();
    let mut ps = makeps(&mut pubclient);
    let msg = ps.get_message().unwrap();
    let payload: String = msg.get_payload().unwrap();
    println!("redis {}", payload);

    let mut client = rdgo().unwrap();
    let json: String = client.lpop("orders").unwrap();
    let data = json::parse(&json).unwrap();
}

fn rdgo() -> Result<redis::Connection, redis::RedisError>{
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let con = client.get_connection()?;
    Ok(con)
}

fn makeps<'a>(c: &'a mut redis::Connection) -> redis::PubSub<'a> {
    let mut ps = c.as_pubsub();
    let _ = ps.subscribe("orders");
    ps
}
