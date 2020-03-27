mod config;
mod email;
mod eth;
mod etherscan;
mod exchange;
mod exchanges;
mod geth;
mod redis;
mod time;
mod types;
mod wallet;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let config_filename = "config.yaml";
    let config = config::read_config(config_filename);
    let exchanges_filename = "exchanges.yaml";
    let exchanges = config::read_exchanges(exchanges_filename);
    let wallet_filename = "wallet.yaml";
    let wallet = wallet::Wallet::load_file(wallet_filename);
    println!("Yith. {:#?} ", config_filename);
    let redis = redis::Redis {
        url: &config.redis_url,
    };
    app(&config, wallet, exchanges, redis, args).unwrap();
}

fn app(
    config: &config::Config,
    mut wallet: wallet::Wallet,
    exchanges: config::ExchangeList,
    redis: redis::Redis,
    args: Vec<String>,
) -> Result<u32, redis::Error> {
    let order: types::Order;

    let my_addr = eth::privkey_to_addr(&config.wallet_private_key);
    println!("etherscan BALANCES for 0x{}", my_addr);
    let mut eth_coins = Vec::<wallet::WalletCoin>::new();
    for coin in wallet.coins.iter() {
        let mut balance = etherscan::balance(&my_addr, &coin.contract, &config.etherscan_key);
        if &coin.ticker_symbol == "ETH" || &coin.ticker_symbol == "WETH_765cc2" {
            balance = eth::wei_to_eth(balance)
        }
        let eth_coin =
            wallet::WalletCoin::build(&coin.ticker_symbol, &coin.contract, &my_addr, balance);
        eth_coins.push(eth_coin);
    }
    wallet.coins.append(&mut eth_coins);
    for exchange in exchanges.enabled() {
        let mut exchange_coins = Vec::<wallet::WalletCoin>::new();
        if exchange.settings.has_balances {
            println!("{} BALANCES for 0x{}", exchange.settings.name, my_addr);
            let balances = exchange.api.balances(&my_addr, &exchange.settings);
            for (symbol, balance) in balances {
                let exchange_coin =
                    wallet::WalletCoin::build(&symbol, "none", &exchange.settings.name, balance);
                exchange_coins.push(exchange_coin);
            }
            wallet.coins.append(&mut exchange_coins);
        }
    }
    println!("{}", wallet);

    for exchange in exchanges.enabled() {
        let orders = exchange
            .api
            .open_orders(&config.wallet_private_key, &exchange.settings);
        println!("{} ORDERS {:?}", exchange.settings.name, orders);
    }
    println!("");

    let order = if args.len() == 2 {
        let arb_filename = args[1].clone();
        println!("loading {}", arb_filename);
        types::Order::from_file(arb_filename)
    } else {
        let mut client = redis::rdsetup(&config.redis_url)?;
        redis.rd_next(&mut client)
    };

    println!(
        "{}/{} Cost {:0.5} Profit {:0.5} {}",
        order.pair.base, order.pair.quote, order.cost, order.profit, order.id,
    );
    run_order(config, &mut wallet, &order, &exchanges);
    Ok(0)
}

fn run_order(
    config: &config::Config,
    wallet: &mut wallet::Wallet,
    order: &types::Order,
    exchanges: &config::ExchangeList,
) {
    let ask_runs = run_books(config, wallet, &order.ask_books, exchanges);
    let bid_runs = run_books(config, wallet, &order.bid_books, exchanges);
    let run_out = format!(
        "order #{}\nask runs: \n{}\n\nbid runs: \n{}",
        order.id,
        format_runs(ask_runs),
        format_runs(bid_runs)
    );
    if let Some(email) = config.email.as_ref() {
        let subject = format!("{}", order.pair);
        email::send(email, &subject, &run_out);
    }
}

fn run_books(
    config: &config::Config,
    wallet: &wallet::Wallet,
    books: &types::Books,
    exchanges: &config::ExchangeList,
) -> Vec<Vec<String>> {
    books.books[..1]
        .iter()
        .map(|book| run_book(config, wallet, &books.askbid, book, exchanges))
        .collect::<Vec<Vec<String>>>()
}

fn run_book(
    config: &config::Config,
    wallet: &wallet::Wallet,
    askbid: &types::AskBid,
    book: &types::Book,
    exchanges: &config::ExchangeList,
) -> Vec<String> {
    book.offers[..1] // first offer
        .iter()
        .map(|offer| {
            println!("** {} {} {}", askbid, &book.market, offer);
            let exchange_name = &book.market.source.name;
            match exchanges.find_by_name(exchange_name) {
                Some(exchange) => {
                    if exchange.settings.enabled {
                        let out =
                            match run_offer(config, askbid, &exchange, offer, &book.market, wallet)
                            {
                                Ok(zip) => {
                                    wait_order(config, &exchange);
                                    "ok".to_string()
                                }
                                Err(e) => e.to_string(),
                            };
                        format!("{} {} {}\n{}", askbid, &book.market, offer, out)
                    } else {
                        println!("exchange {} is disabled!", exchange_name);
                        format!("exchange {} is disabled!", exchange_name)
                    }
                }
                None => format!("exchange detail not found for: {:#?}", exchange_name),
            }
        })
        .collect::<Vec<String>>()
}

fn run_offer(
    config: &config::Config,
    askbid: &types::AskBid,
    exchange: &config::Exchange,
    offer: &types::Offer,
    market: &types::Market,
    wallet: &wallet::Wallet,
) -> Result<(), Box<dyn std::error::Error>> {
    let pub_key = eth::privkey_to_addr(&config.wallet_private_key);
    let (askbid, market, offer) = unswap(askbid, market, offer);
    let source_name = if exchange.settings.has_balances {
        &market.source_name
    } else {
        &pub_key
    };
    let check_ticker = match askbid {
        types::AskBid::Ask => &market.quote,
        types::AskBid::Bid => &market.base,
    };
    match wallet.find_coin_by_source_symbol(source_name, &check_ticker.symbol) {
        Ok(coin) => {
            let wallet_coin_limit = wallet.coin_limit(&check_ticker.symbol);
            let offer_cost = offer.cost(askbid);
            let amounts = vec![offer_cost, wallet_coin_limit, coin.base_total()];
            let least_cost = minimum(amounts);
            if least_cost < offer_cost {
                println!(
                    "{} {} balance limited to {}",
                    check_ticker, source_name, least_cost
                );
            }
            let least_qty = match askbid {
                types::AskBid::Ask => least_cost / offer.quote,
                types::AskBid::Bid => least_cost,
            };
            let capped_offer = types::Offer {
                base_qty: least_qty,
                quote: offer.quote,
            };
            match exchange.api.build(
                &config.wallet_private_key,
                &askbid,
                &exchange.settings,
                &market,
                &capped_offer,
            ) {
                Ok(sheet) => exchange.api.submit(&exchange.settings, sheet),
                Err(e) => Err(e),
            }
        }
        Err(e) => {
            let exg_err = exchange::ExchangeError {
                msg: format!("!{} not found in wallet for {}", check_ticker, source_name),
            };
            println!("{}", exg_err);
            Err(Box::new(exg_err))
        }
    }
}

fn unswap(
    askbid: &types::AskBid,
    market: &types::Market,
    offer: &types::Offer,
) -> (types::AskBid, exchange::Market, types::Offer) {
    let mut quote_token = &market.quote;
    let mut base_token = &market.base;
    let mut qty = offer.base_qty;
    let mut price = offer.quote;
    let mut askbid_align = *askbid; // enum questions
    let askbid_other = askbid.otherside();
    if market.swapped {
        askbid_align = askbid_other;
        quote_token = &market.base;
        base_token = &market.quote;
        let (swap_qty, swap_price) = offer.swap();
        qty = swap_qty;
        price = swap_price;
        println!("unswapped {:#?} {} {}@{}", askbid_align, market, qty, price);
    }
    // market after flip
    let exmarket = exchange::Market {
        base: types::Ticker {
            symbol: base_token.symbol.clone(),
        },
        quote: types::Ticker {
            symbol: quote_token.symbol.clone(),
        },
        quantity_decimals: market.quantity_decimals,
        price_decimals: market.price_decimals,
        source_name: market.source.name.clone(),
    };
    let swoffer = types::Offer {
        base_qty: qty,
        quote: price,
    };
    (askbid_align, exmarket, swoffer)
}

fn wait_order(config: &config::Config, exchange: &config::Exchange) {
    let mut open_orders: Vec<exchange::Order> = vec![];
    while open_orders.len() > 0 {
        open_orders = exchange
            .api
            .open_orders(&config.wallet_private_key, &exchange.settings);
        println!("{} {:?}", exchange.settings.name, open_orders);
        let delay = std::time::Duration::from_secs(3);
        std::thread::sleep(delay);
    }
}

fn minimum(amounts: Vec<f64>) -> f64 {
    amounts
        .iter()
        .fold(std::f64::MAX, |memo, f| if *f < memo { *f } else { memo })
}

fn format_runs(runs: Vec<Vec<String>>) -> String {
    runs.iter()
        .enumerate()
        .fold(String::new(), |mut m, (idx, t)| {
            let line = t.iter().enumerate().fold(String::new(), |mut m, (idx, t)| {
                m.push_str(&format!("offr #{}: {}", idx, t));
                m
            });
            m.push_str(&format!("exg #{}: {}", idx, line));
            m
        })
}
