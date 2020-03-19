mod config;
mod email;
mod eth;
mod etherscan;
mod exchange;
mod exchanges;
mod geth;
mod redis;
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
    println!("etherscan balance warmup for 0x{}", my_addr);
    let mut new_coins = Vec::<wallet::WalletCoin>::new();
    for coin in wallet.coins.iter() {
        let mut balance = etherscan::balance(&my_addr, &coin.contract, &config.etherscan_key);
        if &coin.ticker_symbol == "ETH" || &coin.ticker_symbol == "WETH_765cc2" {
            balance = eth::wei_to_eth(balance)
        }
        new_coins.push(wallet::WalletCoin {
            ticker_symbol: coin.ticker_symbol.clone(),
            contract: coin.contract.clone(),
            source: my_addr.clone(),
            amounts: vec![types::Offer {
                base_qty: balance,
                quote: 1.0,
            }],
        });
    }
    wallet.coins.append(&mut new_coins);
    for exchange in &exchanges.exchanges {
        let mut exchange_coins = Vec::<wallet::WalletCoin>::new();
        let mut coin_symbols = Vec::<&str>::new();
        for coin in wallet.coins.iter() {
            coin_symbols.push(&coin.ticker_symbol);
        }
        if exchange.settings.enabled && exchange.settings.has_balances {
            let balances = exchange
                .api
                .balances(&my_addr, coin_symbols, "wut", &exchange.settings);
            for balance in balances {
                exchange_coins.push(wallet::WalletCoin {
                    ticker_symbol: balance.0.to_string(),
                    contract: "none".to_string(),
                    source: exchange.settings.name.clone(),
                    amounts: vec![types::Offer {
                        base_qty: balance.1,
                        quote: 1.0,
                    }],
                });
            }
        }
        wallet.coins.append(&mut exchange_coins);
    }
    println!("{}", wallet);

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
        "ask runs: \n{}\n\nbid runs: \n{}",
        format_runs(ask_runs),
        format_runs(bid_runs)
    );
    let subject = format!("{}", order.pair);
    email::send(&config.email, &subject, &run_out);
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
    book.offers[..1]
        .iter()
        .map(|offer| {
            // 1 offer limit
            println!("** {} {} {}", askbid, &book.market, offer);
            let exchange_name = &book.market.source.name;
            match exchanges.find_by_name(exchange_name) {
                Some(exchange) => {
                    if exchange.settings.enabled {
                        let out = match run_offer(
                            config,
                            askbid,
                            &exchange,
                            offer,
                            &book.market,
                            wallet,
                        ) {
                            Ok(zip) => "ok".to_string(),
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
    let most_quote = balance_limit(wallet, &market.quote, offer.cost());
    let most_qty = most_quote / offer.quote;
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
        config.proxy.clone(),
    ) {
        Ok(sheet) => exchange.api.submit(&exchange.settings, sheet),
        Err(e) => Err(e),
    }
}

fn balance_limit(wallet: &wallet::Wallet, ticker: &types::Ticker, amount: f64) -> f64 {
    let wallet_coin_balance = wallet.coin_amount(&ticker.symbol);
    if amount < wallet_coin_balance {
        amount
    } else {
        println!(
            "* {} balance capped at {} from {}",
            ticker.symbol, wallet_coin_balance, amount
        );
        wallet_coin_balance
    }
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
