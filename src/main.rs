use redis::{Commands, RedisError};

mod config;
mod error;
mod eth;
mod etherscan;
mod exchange;
mod exchanges;
mod geth;
mod types;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_filename = "config.yaml";
    let config = config::read_config(config_filename);
    let exchanges_filename = "exchanges.yaml";
    let exchanges = config::read_exchanges(exchanges_filename);
    let wallet_filename = "wallet.yaml";
    let wallet = config::read_wallet(wallet_filename);
    println!("Yith. {:#?} ", config_filename);
    println!("{}", wallet);
    app(&config, &wallet, exchanges, args).unwrap();
}

fn app(
    config: &config::Config,
    wallet: &config::Wallet,
    exchanges: config::ExchangeList,
    args: Vec<String>,
) -> Result<u32, RedisError> {
    let mut arb_id: String;
    let mut order: types::Order;

    let my_addr = eth::privkey_to_addr(&config.wallet_private_key);
    for coin in &wallet.coins {
        let balance = etherscan::balance(&my_addr, &coin.contract, &config.etherscan_key);
        println!("{} {:0.4}", &coin.ticker_symbol, &balance / 10_f64.powi(18));
    }

    if args.len() == 2 {
        arb_id = args[1].clone();
        println!("loading {}", arb_id);
        order = fd_order(arb_id);
    } else {
        let mut client = rdsetup(&config.redis_url)?;
        let inplay_exists = client.exists("inplay")?;
        arb_id = match inplay_exists {
            true => {
                println!("active order found!");
                rd_inplay(&mut client)
            }
            false => {
                println!("no active order. waiting for order.");
                rd_next_order(config)
            }
        }?;
        order = rd_order(&mut client, arb_id)?;
    }

    println!(
        "Order {} loaded. Cost {} Profit {}",
        order.id, order.cost, order.profit
    );
    run_order(config, wallet, &order, &exchanges);
    Ok(0)
}

fn run_order(
    config: &config::Config,
    wallet: &config::Wallet,
    order: &types::Order,
    exchanges: &config::ExchangeList,
) {
    println!("{}/{}:", &order.pair.base, &order.pair.quote);
    run_books(config, wallet, &order.ask_books, exchanges);
    //run_books(config, wallet, &order.bid_books, exchanges);
}

fn run_books(
    config: &config::Config,
    wallet: &config::Wallet,
    books: &types::Books,
    exchanges: &config::ExchangeList,
) {
    for book in &books.books[..1] {
        let wallet_coin_balance = wallet.coin_amount(&book.market.quote.symbol);
        for offer in &book.offers[..1] {
            // limit to one
            let exchange_name = &book.market.source.name;
            println!("{:?} {}", &books.askbid, exchange_name);
            let most_qty = if offer.base_qty < wallet_coin_balance {
                offer.base_qty
            } else {
                println!(
                    "Offer {} capped at {} {}",
                    offer, wallet_coin_balance, book.market.quote
                );
                wallet_coin_balance
            };
            let capped_offer = types::Offer {
                base_qty: most_qty,
                quote: offer.quote,
            };
            let _:Result<exchange::OrderSheet, std::boxed::Box<dyn std::error::Error>> = match exchanges.find_by_name(exchange_name) {
                Some(exg) => exg.api.build(
                    &config.wallet_private_key,
                    &books.askbid,
                    &exg.settings,
                    &book.market,
                    &capped_offer,
                    &config.proxy,
                ),
                None => {
                    println!("exchange not found for: {:#?}", exchange_name);
                    Err(Box::new(error::OrderError::new("not found")))
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

fn fd_order(arb_id: String) -> types::Order {
    let filename = format!("arbs/{}/order", arb_id);
    let json = std::fs::read_to_string(filename).expect("order json file bad");
    let order: types::Order = serde_yaml::from_str(&json).unwrap();
    order
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
