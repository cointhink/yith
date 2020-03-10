mod config;
mod error;
mod eth;
mod etherscan;
mod exchange;
mod exchanges;
mod geth;
mod redis;
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
) -> Result<u32, redis::Error> {
    let order: types::Order;

    let my_addr = eth::privkey_to_addr(&config.wallet_private_key);
    for coin in &wallet.coins {
        let balance = etherscan::balance(&my_addr, &coin.contract, &config.etherscan_key);
        println!("{} {:0.4}", &coin.ticker_symbol, &balance / 10_f64.powi(18));
    }

    if args.len() == 2 {
        let arb_filename = args[1].clone();
        println!("loading {}", arb_filename);
        order = types::Order::from_file(arb_filename);
    } else {
        let mut client = redis::rdsetup(&config.redis_url)?;
        let inplay_exists = redis::rd_exists(&mut client, "inplay");
        let arb_id = match inplay_exists {
            true => {
                println!("active order found!");
                redis::rd_inplay(&mut client)
            }
            false => {
                println!("no active order. waiting for order.");
                redis::rd_next_order(config)
            }
        }?;
        order = redis::rd_order(&mut client, arb_id)?;
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
    run_books(config, wallet, &order.bid_books, exchanges);
}

fn run_books(
    config: &config::Config,
    wallet: &config::Wallet,
    books: &types::Books,
    exchanges: &config::ExchangeList,
) {
    for book in &books.books[..1] {
        for offer in &book.offers[..1] {
            // 1 offer limit
            let exchange_name = &book.market.source.name;
            println!("{:?} {}", &books.askbid, exchange_name);
            match exchanges.find_by_name(exchange_name) {
                Some(exchange) => run_offer(
                    config,
                    &books.askbid,
                    &exchange,
                    offer,
                    &book.market,
                    wallet,
                )
                .unwrap(),
                None => {
                    println!("exchange detail not found for: {:#?}", exchange_name);
                }
            }
        }
    }
}

fn run_offer(
    config: &config::Config,
    askbid: &types::AskBid,
    exchange: &config::Exchange,
    offer: &types::Offer,
    market: &types::Market,
    wallet: &config::Wallet,
) -> Result<(), Box<dyn std::error::Error>> {
    let most_qty = balance_check(wallet, exchange, &market.quote, offer.base_qty);
    let capped_offer = types::Offer {
        base_qty: most_qty,
        quote: offer.quote,
    };
    match exchange.api.build(
        &config.wallet_private_key,
        &askbid,
        &exchange.settings,
        market,
        &capped_offer,
        &config.proxy,
    ) {
        Ok(sheet) => exchange.api.submit(sheet),
        Err(e) => {
            println!("{}", e);
            Err(Box::new(error::OrderError::new(&e.description())))
        }
    }
}

fn balance_check(
    wallet: &config::Wallet,
    exchange: &config::Exchange,
    ticker: &types::Ticker,
    amount: f64,
) -> f64 {
    let wallet_coin_balance = wallet.coin_amount(&ticker.symbol);
    if amount < wallet_coin_balance {
        amount
    } else {
        println!(
            "** {} balance capped at {} from {}",
            ticker.symbol, wallet_coin_balance, amount);
        wallet_coin_balance
    }
}
