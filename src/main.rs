use clap;

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
    if let Some(_matches) = opts.subcommand_matches("balances") {
        load_wallet(&mut wallet.coins, &exchanges, &config);
        println!("{}", wallet);
    }
    if let Some(_matches) = opts.subcommand_matches("open") {
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

        //show_orders(&exchanges, &config.wallet_private_key);
        //println!("");

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

        let order = build_manual_order(matches);
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

fn run_order(
    config: &config::Config,
    wallet: &mut wallet::Wallet,
    order: &types::Order,
    exchanges: &config::ExchangeList,
) {
    let ask_sheets = build_books(config, wallet, &order.ask_books, exchanges, Mode::Real);
    let ask_sheets_out = format_runs(&ask_sheets);
    let ask_sheets_len = count_sheets(&ask_sheets);
    let ask_goods = filter_good_sheets(ask_sheets);
    let ask_goods_len = count_sheets(&ask_goods);
    println!("a {}/{}", ask_goods_len, ask_sheets_len);
    let mut run_out = "".to_string();

    if ask_goods_len == ask_sheets_len {
        let sim_bid_sheets =
            build_books(config, wallet, &order.bid_books, exchanges, Mode::Simulate);
        let sim_bid_sheets_out = format_runs(&sim_bid_sheets);
        let sim_bid_sheets_len = count_sheets(&sim_bid_sheets);
        let sim_bid_goods = filter_good_sheets(sim_bid_sheets);
        let sim_bid_goods_len = count_sheets(&sim_bid_goods);
        println!("sb {}/{}", sim_bid_goods_len, sim_bid_sheets_len);

        if sim_bid_goods_len == sim_bid_sheets_len {
            let ask_runs = run_sheets(config, ask_goods, exchanges);

            let bid_sheets = build_books(config, wallet, &order.bid_books, exchanges, Mode::Real);
            let bid_sheets_out = format_runs(&bid_sheets);
            let bid_sheets_len = count_sheets(&bid_sheets);
            let bid_goods = filter_good_sheets(bid_sheets);
            let bid_goods_len = count_sheets(&bid_goods);
            println!("b {}/{}", bid_goods_len, bid_sheets_len);

            if bid_goods_len == bid_sheets_len {
                let bid_runs = run_sheets(config, bid_goods, exchanges);
                run_out = format!(
                    "ask runs: \n{}\n\nsim bid runs: \n{}\n\nbid runs: \n{}",
                    ask_sheets_out, sim_bid_sheets_out, bid_sheets_out,
                );
            } else {
                println!(
                    "sumbit aborted! bids {} good {} (thats bad)",
                    bid_sheets_len, bid_goods_len
                );
            }
        } else {
            println!(
                "submit aborted! sim_bid {} good {} (thats bad)",
                sim_bid_sheets_len, sim_bid_goods_len
            );
        }
    } else {
        println!(
            "submit aborted! asks {} good {} (thats bad)",
            ask_sheets_len, ask_goods_len
        );
    }

    if let Some(email) = config.email.as_ref() {
        let subject = format!("{}", order.pair);
        let out = format!(
            "order #{} {} {} {}\n{}",
            order.id, order.pair, order.cost, order.profit, run_out
        );
        email::send(email, &subject, &out);
    }
}

fn filter_good_sheets(
    sheets: Vec<(
        String,
        Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
    )>,
) -> Vec<(String, Vec<exchange::OrderSheet>)> {
    sheets
        .into_iter()
        .map(|(exchange_name, t)| {
            let good_sheets =
                t.into_iter()
                    .fold(
                        Vec::<exchange::OrderSheet>::new(),
                        |mut memo, result| match result {
                            Ok(sheet) => {
                                memo.push(sheet);
                                memo
                            }
                            Err(_e) => memo,
                        },
                    );
            (exchange_name, good_sheets)
        })
        .collect()
}

fn count_sheets<T>(sheets: &Vec<(String, Vec<T>)>) -> usize {
    sheets.iter().fold(0, |m, s| m + s.1.len())
}

// rust to learn
#[derive(Clone, Copy)]
enum Mode {
    Simulate,
    Real,
}

fn build_books(
    config: &config::Config,
    wallet: &wallet::Wallet,
    books: &types::Books,
    exchanges: &config::ExchangeList,
    mode: Mode,
) -> Vec<(
    String,
    Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
)> {
    books
        .books
        .iter()
        .take(1) // first offer
        .map(|book| {
            let exchange_name = book.market.source.name.clone();
            match exchanges.find_by_name(&exchange_name) {
                Some(exchange) => {
                    if exchange.settings.enabled {
                        (
                            exchange_name,
                            build_book(config, wallet, &books.askbid, book, exchange, mode),
                        )
                    } else {
                        // rust to learn
                        let borrow_check_omg = exchange_name.clone();
                        (
                            exchange_name,
                            vec![Err(exchange::ExchangeError::build_box(format!(
                                "exchange {} is disabled!",
                                borrow_check_omg
                            )))],
                        )
                    }
                }
                None => (
                    "exchange-missing".to_string(),
                    vec![Err(exchange::ExchangeError::build_box(format!(
                        "exchange detail not found for: {:#?}",
                        exchange_name
                    )))],
                ),
            }
        })
        .collect()
}

fn build_book(
    config: &config::Config,
    wallet: &wallet::Wallet,
    askbid: &types::AskBid,
    book: &types::Book,
    exchange: &config::Exchange,
    mode: Mode,
) -> Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>> {
    book.offers
        .iter()
        .take(1) // first offer
        .map(|offer| {
            println!(
                "** {} {} {} {} => {}{}",
                match mode {
                    Mode::Real => "BUILD",
                    Mode::Simulate => "SIMBUILD",
                },
                askbid,
                &book.market,
                offer,
                offer.cost(*askbid),
                &book.market.quote
            );
            build_offer(config, askbid, &exchange, offer, &book.market, wallet, mode)
        })
        .collect()
}

fn build_offer(
    config: &config::Config,
    askbid: &types::AskBid,
    exchange: &config::Exchange,
    offer: &types::Offer,
    market: &types::Market,
    wallet: &wallet::Wallet,
    mode: Mode,
) -> Result<exchange::OrderSheet, Box<dyn std::error::Error>> {
    println!("build offer {} {}", exchange, offer);
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
    let mut amount_limits = vec![];
    match wallet.find_coin_by_source_symbol(source_name, &check_ticker.symbol) {
        Ok(coin) => {
            amount_limits.push(coin.base_total());
        }
        Err(_e) => {
            let err = exchange::ExchangeError::build_box(format!(
                "WARNING: {} balance unknown for {}",
                check_ticker, source_name
            ));
            match mode {
                Mode::Simulate => (),
                Mode::Real => return Err(err), // early return
            }
        }
    };
    match wallet.find_coin_by_source_symbol("limit", &check_ticker.symbol) {
        Ok(coin) => {
            let wallet_coin_limit = wallet.coin_limit(&check_ticker.symbol);
            amount_limits.push(wallet_coin_limit);
            amount_limits.push(coin.base_total());
        }
        Err(_e) => {
            let _err = exchange::ExchangeError::build_box(format!(
                "WARNING: {} wallet limit not set",
                check_ticker
            ));
        }
    };

    let offer_cost = offer.cost(askbid);
    amount_limits.push(offer_cost);
    let least_cost = minimum(amount_limits);
    let least_qty = match askbid {
        types::AskBid::Ask => least_cost / offer.quote,
        types::AskBid::Bid => least_cost,
    };
    if least_cost < offer_cost {
        exchange::ExchangeError::build_box(format!(
            "{} {} balance capped at {:0.4}. adj qty {:0.4}",
            check_ticker, source_name, least_cost, least_qty
        ));
    }

    let market_min_opt = exchange
        .api
        .market_minimum(check_ticker, &exchange.settings);
    match market_min_opt {
        Some(market_minimum) => {
            if market_minimum > least_cost {
                let err = exchange::ExchangeError::build_box(format!(
                    "{} minimum of {:0.4} NOT met with {:0.4}{}",
                    &market, market_minimum, least_cost, check_ticker
                ));
                return Err(err);
            } else {
                println!(
                    "{} minimum {}{} met with {}{}",
                    &market, market_minimum, check_ticker, least_cost, &market.quote
                );
            }
        }
        None => (),
    }

    match mode {
        Mode::Real => {
            let capped_offer = types::Offer {
                base_qty: least_qty,
                quote: offer.quote,
            };
            exchange.api.build(
                &config.wallet_private_key,
                &askbid,
                &exchange.settings,
                &market,
                &capped_offer,
            )
        }
        Mode::Simulate => Ok(exchange::OrderSheet::Placebo),
    }
}

fn run_sheets(
    config: &config::Config,
    sheets: Vec<(String, Vec<exchange::OrderSheet>)>,
    exchanges: &config::ExchangeList,
) {
    sheets.into_iter().for_each(|(en, t)| {
        t.into_iter().for_each(move |sheet| {
            let _m = match exchanges.find_by_name(&en) {
                Some(exchange) => run_sheet(config, sheet, exchange),
                None => Ok(()),
            };
        });
    });
}

fn run_sheet(
    config: &config::Config,
    sheet: exchange::OrderSheet,
    exchange: &config::Exchange,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("** RUN sheet {} {:?}", exchange, sheet);
    match exchange
        .api
        .submit(&config.wallet_private_key, &exchange.settings, sheet)
    {
        Ok(_sheet) => {
            wait_order(config, &exchange);
            Ok(())
        }
        Err(e) => Err(e),
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

fn format_runs(
    runs: &Vec<(
        String,
        Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
    )>,
) -> String {
    runs.iter()
        .enumerate()
        .fold(String::new(), |mut m, (idx, (en, t))| {
            let line = t.iter().enumerate().fold(String::new(), |mut m, (idx, r)| {
                let out = match r {
                    Ok(sheet) => format!("offr #{}: {} {:?}", idx, en, sheet),
                    Err(err) => err.to_string(),
                };
                m.push_str(&out);
                m
            });
            m.push_str(&format!("exg #{}: {}", idx, line));
            m
        })
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
    let escan = etherscan::Etherscan::new();
    let mut coins = Vec::<wallet::WalletCoin>::new();
    for coin in wallet_coins {
        let mut balance = etherscan::balance(my_addr, &coin.contract, api_key);
        let token = types::Ticker {
            symbol: coin.ticker_symbol.clone(),
        };
        let decimals = match escan.tokens.get(&token) {
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

fn build_manual_order(matches: &clap::ArgMatches) -> types::Order {
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
        "build {} {} {}{}@{}{}",
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
        unknown => println!("pick buy/sell: {}", unknown),
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
