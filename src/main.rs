use redis::{Commands, RedisError};

mod config;
mod eth;
mod exchanges;
mod geth;
mod types;

fn main() {
    let config_filename = "config.yaml";
    let exchanges_filename = "exchanges.yaml";
    let config = config::read_config(config_filename);
    let exchanges = config::read_exchanges(exchanges_filename);
    println!("Yith. {:#?} ", config_filename);
    //geth::rpc(&config, &config.geth_url, "eth_gasPrice");
    app(&config, exchanges).unwrap();
}

fn app(config: &config::Config, exchanges: config::ExchangeList) -> Result<u32, RedisError> {
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

    let order: types::Order = rd_order(&mut client, arb_id)?;
    println!(
        "Order {} loaded. Cost {} Profit {}",
        order.id, order.cost, order.profit
    );
    run_order(config, &order, &exchanges);
    Ok(0)
}

fn run_order(config: &config::Config, order: &types::Order, exchanges: &config::ExchangeList) {
    println!("{}/{}:", &order.pair.base, &order.pair.quote);
    run_books(config, &order.ask_books, exchanges);
    run_books(config, &order.bid_books, exchanges);
}

fn run_books(config: &config::Config, books: &types::Books, exchanges: &config::ExchangeList) {
    for book in &books.books {
        for offer in &book.offers {
            let exchange_name = &book.market.source.name;
            match exchanges.find_by_name(exchange_name) {
                Some(exg) => match exg.protocol {
                    config::ExchangeProtocol::ZeroexOpen => exchanges::zeroex::build(
                        &config.wallet_private_key,
                        &books.askbid,
                        exg,
                        &book.market,
                        &offer,
                        &config.proxy,
                    ),
                    config::ExchangeProtocol::Ddex3 => exchanges::ddex3::build(
                        &config.wallet_private_key,
                        &books.askbid,
                        exg,
                        &book.market,
                        &offer,
                        &config.proxy,
                    ),
                },
                None => {
                    println!("exchange not found for: {:#?}", exchange_name);
                    Ok(())
                }
            };
        }
    }
}

fn rd_order(client: &mut redis::Connection, arb_id: String) -> Result<types::Order, RedisError> {
    let hkey = [String::from("arb:"), arb_id].concat();
    let json: String = client.hget(&hkey, "json")?;
    let order: types::Order = serde_yaml::from_str(&json).unwrap();
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

fn rd_next_order(config: &config::Config) -> Result<String, redis::RedisError> {
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
