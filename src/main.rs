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
    let opt_yaml = clap::load_yaml!("cli.yaml"); // load/parse at compile time
    let opt_matches = clap::App::from_yaml(opt_yaml).get_matches();
    let config_filename = opt_matches.value_of("config").unwrap_or("config.yaml");
    let config = config::read_config(config_filename)
        .unwrap_or_else(|c| panic!("{} {:?}", config_filename, c.to_string()));
    let exchanges_filename = "exchanges.yaml";
    let exchanges = config::read_exchanges(exchanges_filename, &config)
        .unwrap_or_else(|c| panic!("{} {:?}", exchanges_filename, c.to_string()));
    let wallet_filename = "wallet.yaml";
    let wallet = wallet::Wallet::load_file(wallet_filename)
        .unwrap_or_else(|c| panic!("{} {:?}", wallet_filename, c.to_string()));
    println!("Yith {:#?} ", config_filename);
    let redis = redis::Redis {
        url: &config.redis_url,
    };
    app(&config, wallet, exchanges, redis, opt_matches).unwrap();
}

fn app(
    config: &config::Config,
    mut wallet: wallet::Wallet,
    exchanges: config::ExchangeList,
    redis: redis::Redis,
    opts: clap::ArgMatches,
) -> Result<u32, Box<dyn std::error::Error>> {
    if let Some(matches) = opts.subcommand_matches("balances") {
        load_wallet(&mut wallet.coins, &exchanges, &config);
        println!("{}", wallet);
    }
    if let Some(matches) = opts.subcommand_matches("open") {
        show_orders(&exchanges, &config.wallet_private_key);
    }
    if let Some(matches) = opts.subcommand_matches("withdrawl") {
        let exchange = exchanges.find_by_name(matches.value_of("exchange").unwrap());
        let amount = matches.value_of("amount").unwrap().parse::<f64>().unwrap();
        let symbol = matches.value_of("token").unwrap();
        let token = types::Ticker {
            symbol: symbol.to_uppercase(),
        };
        match exchange {
            Some(exchange) => {
                exchange.api.withdrawl(
                    &config.wallet_private_key,
                    &exchange.settings,
                    amount,
                    token,
                );
            }
            None => println!("exchange not found"),
        }
    }

    if let Some(matches) = opts.subcommand_matches("run") {
        load_wallet(&mut wallet.coins, &exchanges, &config);
        println!("{}", wallet);

        show_orders(&exchanges, &config.wallet_private_key);
        println!("");

        let order = match matches.value_of("arb_file") {
            Some(filename) => {
                println!("loading {}", filename);
                types::Order::from_file(filename.to_string())
            }
            None => {
                let mut client = redis::rdsetup(&config.redis_url)?;
                redis.rd_next(&mut client)
            }
        };

        println!(
            "{}/{} Cost {:0.5} Profit {:0.5} {}",
            order.pair.base, order.pair.quote, order.cost, order.profit, order.id,
        );
        run_order(config, &mut wallet, &order, &exchanges);
    }
    if let Some(matches) = opts.subcommand_matches("order") {
        load_wallet(&mut wallet.coins, &exchanges, &config);
        println!("{}", wallet);

        let order = build_order(matches);
        run_order(config, &mut wallet, &order, &exchanges);
    }
    Ok(0)
}

fn load_wallet(
    coins: &mut Vec<wallet::WalletCoin>,
    exchanges: &config::ExchangeList,
    config: &config::Config,
) {
    let my_addr = eth::privkey_to_addr(&config.wallet_private_key);
    println!("etherscan BALANCES for 0x{}", my_addr);
    let mut eth_coins = etherscan_coins(&my_addr, coins, &config.etherscan_key);
    coins.append(&mut eth_coins);
    for exchange in exchanges.enabled() {
        let mut exchange_coins = exchange_coins(&my_addr, exchange);
        coins.append(&mut exchange_coins);
    }
}

fn show_orders(exchanges: &config::ExchangeList, private_key: &str) {
    for exchange in exchanges.enabled() {
        let orders = exchange.api.open_orders(private_key, &exchange.settings);
        println!("{} {} ORDERS", exchange.settings.name, orders.len());
        for order in orders {
            println!(
                "  {} {:?} {} {} {:.5}@{:.5} {}",
                order.id,
                order.state,
                order.side,
                order.market,
                order.base_qty,
                order.quote,
                &order.create_date[0..10],
            );
        }
    }
}

fn build_order(matches: &clap::ArgMatches) -> types::Order {
    let exchange = matches.value_of("exchange").unwrap();
    let side = matches.value_of("side").unwrap();
    let quantity_str = matches.value_of("quantity").unwrap();
    let quantity = quantity_str.parse::<f64>().unwrap();
    let base_symbol = matches.value_of("base_token").unwrap();
    let ask_base = types::Ticker {
        symbol: base_symbol.to_uppercase(),
    };
    let bid_base = types::Ticker {
        symbol: base_symbol.to_uppercase(),
    };
    let price_str = matches.value_of("price").unwrap();
    let price = price_str.parse::<f64>().unwrap();
    let quote_symbol = matches.value_of("quote_token").unwrap();
    let ask_quote = types::Ticker {
        symbol: quote_symbol.to_uppercase(),
    };
    let bid_quote = types::Ticker {
        symbol: quote_symbol.to_uppercase(),
    };
    println!(
        "{} {} {}{}@{}{}",
        exchange, side, quantity, base_symbol, price, quote_symbol
    );

    let pair = types::Pair {
        base: base_symbol.to_string(),
        quote: quote_symbol.to_string(),
    };
    let offer = types::Offer {
        base_qty: quantity,
        quote: price,
    };
    let ask_source = types::Source {
        name: exchange.to_string(),
    };
    let bid_source = types::Source {
        name: exchange.to_string(),
    };
    let ask_market = types::Market {
        source: ask_source,
        base: ask_base,
        base_contract: "".to_string(),
        quote: ask_quote,
        quote_contract: "".to_string(),
        min_order_size: "0".to_string(),
        price_decimals: 0.0,
        quantity_decimals: 0.0,
        swapped: false,
    };
    let bid_market = types::Market {
        source: bid_source,
        base: bid_base,
        base_contract: "".to_string(),
        quote: bid_quote,
        quote_contract: "".to_string(),
        min_order_size: "0".to_string(),
        price_decimals: 0.0,
        quantity_decimals: 0.0,
        swapped: false,
    };
    let ask_book = types::Book {
        market: ask_market,
        offers: vec![],
    };
    let bid_book = types::Book {
        market: bid_market,
        offers: vec![],
    };
    let mut asks = types::Books {
        askbid: types::AskBid::Ask,
        books: vec![ask_book],
    };
    let mut bids = types::Books {
        askbid: types::AskBid::Bid,
        books: vec![bid_book],
    };
    match side {
        "buy" => asks.books[0].offers.push(offer),
        "sell" => bids.books[0].offers.push(offer),
        _ => {}
    }

    types::Order {
        id: "manual".to_string(),
        date: "now".to_string(),
        pair: pair,
        cost: quantity * price,
        profit: 0.0,
        avg_price: 0.0,
        ask_books: asks,
        bid_books: bids,
    }
}

fn exchange_coins(my_addr: &str, exchange: &config::Exchange) -> Vec<wallet::WalletCoin> {
    let mut exchange_coins = Vec::<wallet::WalletCoin>::new();
    if exchange.settings.has_balances {
        println!("{} BALANCES for 0x{}", exchange.settings.name, my_addr);
        let balances = exchange.api.balances(&my_addr, &exchange.settings);
        for (symbol, balance) in balances {
            let exchange_coin =
                wallet::WalletCoin::build(&symbol, "none", &exchange.settings.name, balance);
            exchange_coins.push(exchange_coin);
        }
    }
    exchange_coins
}

fn etherscan_coins(
    my_addr: &str,
    wallet_coins: &Vec<wallet::WalletCoin>,
    api_key: &str,
) -> Vec<wallet::WalletCoin> {
    let token_list = config::read_tokens("./notes/etherscan-tokens.json");
    let mut coins = Vec::<wallet::WalletCoin>::new();
    for coin in wallet_coins.iter() {
        let mut balance = etherscan::balance(my_addr, &coin.contract, api_key);
        let token = types::Ticker {
            symbol: coin.ticker_symbol.clone(),
        };
        let decimals = match token_list.get(&token) {
            Some(token_detail) => token_detail.decimals,
            None => 0,
        };
        balance = eth::wei_to_eth(balance, decimals);
        let eth_coin =
            wallet::WalletCoin::build(&coin.ticker_symbol, &coin.contract, &my_addr, balance);
        coins.push(eth_coin);
    }
    coins
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
        "order #{} {} {} {}\nask runs: \n{}\n\nbid runs: \n{}",
        order.id,
        order.pair,
        order.cost,
        order.profit,
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
    books
        .books
        .iter()
        .take(1) // first offer
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
    book.offers
        .iter()
        .take(1) // first offer
        .map(|offer| {
            println!(
                "** {} {} {} => {}{}",
                askbid,
                &book.market,
                offer,
                offer.cost(*askbid),
                &book.market.quote
            );
            let exchange_name = &book.market.source.name;
            match exchanges.find_by_name(exchange_name) {
                Some(exchange) => {
                    if exchange.settings.enabled {
                        let out =
                            match run_offer(config, askbid, &exchange, offer, &book.market, wallet)
                            {
                                Ok(()) => {
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
            let least_qty = match askbid {
                types::AskBid::Ask => least_cost / offer.quote,
                types::AskBid::Bid => least_cost,
            };
            if least_cost < offer_cost {
                println!(
                    "{} {} balance capped at {}. adj qty {}",
                    check_ticker, source_name, least_cost, least_qty
                );
            }
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
                Ok(sheet) => {
                    exchange
                        .api
                        .submit(&config.wallet_private_key, &exchange.settings, sheet)
                }
                Err(e) => Err(e),
            }
        }
        Err(_e) => {
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
    let mut quote_contract = &market.quote_contract;
    let mut base_token = &market.base;
    let mut base_contract = &market.base_contract;
    let mut qty = offer.base_qty;
    let mut price = offer.quote;
    let mut askbid_align = *askbid; // enum questions
    let askbid_other = askbid.otherside();
    if market.swapped {
        askbid_align = askbid_other;
        quote_token = &market.base;
        quote_contract = &market.base_contract;
        base_token = &market.quote;
        base_contract = &market.quote_contract;
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
        base_contract: base_contract.clone(),
        quote: types::Ticker {
            symbol: quote_token.symbol.clone(),
        },
        quote_contract: quote_contract.clone(),
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
