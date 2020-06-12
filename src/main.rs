use clap;

mod config;
mod email;
mod erc20;
mod errors;
mod eth;
mod etherscan;
mod exchange;
mod exchanges;
mod geth;
mod log;
mod price;
mod redis;
mod time;
mod types;
mod wallet;
mod weth;

fn main() {
    let options_yaml = clap::load_yaml!("cli.yaml"); // load/parse at compile time
    let options = clap::App::from_yaml(options_yaml).get_matches();

    let config_filename = options.value_of("config").unwrap_or(config::FILENAME);
    let config: config::Config = config::read_type(config_filename);

    let exchanges_filename = "exchanges.yaml";
    let exchanges = config::hydrate_exchanges(exchanges_filename, &config)
        .unwrap_or_else(|c| panic!("{} {}", exchanges_filename, c));

    let wallet_filename = "wallet.yaml";
    let wallet: wallet::Wallet = config::read_type(wallet_filename);

    config::CONFIG.set(config).unwrap(); // set-once global

    println!("Yith {:#?} {}", config_filename, time::now_string());
    match app(wallet, exchanges, options) {
        None => {
            std::process::exit(0);
        }
        Some(err) => {
            eprintln!("stderr: {}", err);
            std::process::exit(1);
        }
    }
}

fn app(
    mut wallet: wallet::Wallet,
    exchanges: config::ExchangeList,
    opts: clap::ArgMatches,
) -> Option<Box<dyn std::error::Error>> {
    let config = config::CONFIG.get().unwrap();

    if let Some(_matches) = opts.subcommand_matches("balances") {
        scan_wallet(&mut wallet.coins, &exchanges);
        wallet.print_with_price();
        None
    } else if let Some(_matches) = opts.subcommand_matches("orders") {
        show_orders(&exchanges, &config.wallet_private_key);
        None
    } else if let Some(matches) = opts.subcommand_matches("weth") {
        let action = matches.value_of("action").unwrap();
        let amount = matches.value_of("amount").unwrap().parse::<f64>().unwrap();
        let geth = geth::Client::build_infura(&config.infura_project_id);
        let amount_str = exchange::quantity_in_base_units(amount, 18, 18).to_string();
        match action {
            "wrap" => {
                weth::Weth::wrap(
                    geth,
                    &config.wallet_private_key,
                    weth::Direction::Wrap,
                    &amount_str,
                );
                None
            }
            "unwrap" => {
                weth::Weth::wrap(
                    geth,
                    &config.wallet_private_key,
                    weth::Direction::Unwrap,
                    &amount_str,
                );
                None
            }
            _ => None,
        }
    } else if let Some(matches) = opts.subcommand_matches("erc20") {
        let action = matches.value_of("action").unwrap();
        let token = matches.value_of("token").unwrap();
        let exchange_name = matches.value_of("exchange").unwrap();
        let exchange = exchanges.find_by_name(exchange_name).unwrap();
        let geth = geth::Client::build_infura(&config.infura_project_id);
        match action {
            "allowance" => {
                let allowance = erc20::Erc20::allowance(
                    geth,
                    &config.wallet_private_key,
                    &token,
                    &exchange.settings.contract_address,
                )
                .unwrap();
                println!("erc20 {} {} {}", token, exchange_name, allowance);
            }
            "approve" => {
                let approve = erc20::Erc20::approve(
                    geth,
                    &config.wallet_private_key,
                    &token,
                    &exchange.settings.contract_address,
                )
                .unwrap();
                println!("erc20 {} {} {}", token, exchange_name, approve);
            }
            _ => (),
        }
        None
    } else if let Some(matches) = opts.subcommand_matches("transfer") {
        let direction_str = matches.value_of("direction").unwrap();
        let direction = match exchange::TransferDirection::read(direction_str) {
            Some(dir) => dir,
            None => {
                return Some(errors::MainError::build_box(format!(
                    "bad transfer direction"
                )))
            }
        };
        //let direction = matches.value_of("direction").unwrap().into();
        let amount = matches.value_of("amount").unwrap().parse::<f64>().unwrap();
        let symbol = matches.value_of("token").unwrap();
        let exchange_name = matches.value_of("exchange").unwrap();
        let exchange = exchanges.find_by_name(exchange_name).unwrap();

        run_transfer(
            &config.wallet_private_key,
            direction,
            &exchange,
            amount,
            &symbol.into(),
        )
    } else if let Some(matches) = opts.subcommand_matches("trade") {
        scan_wallet(&mut wallet.coins, &exchanges);
        wallet.print_with_price();

        let order = build_manual_order(matches);
        let run_log = run_order(config, &mut wallet, &order, &exchanges);
        if let Some(email) = config.email.as_ref() {
            mail_log(&email, &order, &run_log)
        }
        None
    } else if let Some(matches) = opts.subcommand_matches("run") {
        scan_wallet(&mut wallet.coins, &exchanges);
        wallet.print_with_price();

        let order = match matches.value_of("arb_file") {
            Some(filename) => {
                println!("loading {}", filename);
                types::Order::from_file(filename.to_string())
            }
            None => {
                let mut redis = redis::Redis::new(&config.redis_url);
                redis.rd_next()
            }
        };

        let run_log = run_order(config, &mut wallet, &order, &exchanges);
        if let Some(email) = &config.email {
            mail_log(&email, &order, &run_log)
        }
        None
    } else {
        Some(errors::MainError::build_box(format!(
            "option not understood"
        )))
    }
}

fn run_transfer(
    private_key: &str,
    direction: exchange::TransferDirection,
    exchange: &config::Exchange,
    amount: f64,
    token: &types::Ticker,
) -> Option<Box<dyn std::error::Error>> {
    match direction {
        exchange::TransferDirection::Withdrawal => {
            exchange
                .api
                .withdrawl(private_key, &exchange.settings, amount, token);
            None
        }
        exchange::TransferDirection::Deposit => {
            exchange
                .api
                .deposit(private_key, &exchange.settings, amount, token);
            None
        }
    }
}

fn scan_wallet(coins: &mut Vec<wallet::WalletCoin>, exchanges: &config::ExchangeList) {
    let config = config::CONFIG.get().unwrap();
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
) -> log::RunLog {
    let mut run_out = log::RunLog::new();
    run_out.add(format!(
        "{}/{} Cost {:0.5} Profit {:0.5} {}",
        order.pair.base, order.pair.quote, order.cost, order.profit, order.id,
    ));

    let ask_sheets = build_books(config, wallet, &order.ask_books, exchanges, Mode::Real);
    run_out.add(format!("ask builds: \n{}", format_runs(&ask_sheets)));
    let ask_sheets_badlen = count_bad_sheets(&ask_sheets);

    if ask_sheets_badlen > 0 {
        let sim_bid_sheets =
            build_books(config, wallet, &order.bid_books, exchanges, Mode::Simulate);
        run_out.add(format!("simbid builds: \n{}", format_runs(&sim_bid_sheets)));
        let sim_bid_sheets_badlen = count_bad_sheets(&sim_bid_sheets);

        if sim_bid_sheets_badlen > 0 {
            let _ask_runs = run_sheets(config, ask_sheets);
            run_out.add(format!("ask runs: (logging not implemented)\n"));

            // wallet refresh
            wallet.reset();
            scan_wallet(&mut wallet.coins, &exchanges);

            let bid_sheets = build_books(config, wallet, &order.bid_books, exchanges, Mode::Real);
            run_out.add(format!("bid builds: \n{}", format_runs(&bid_sheets)));
            let bid_sheets_badlen = count_bad_sheets(&bid_sheets);

            if bid_sheets_badlen > 0 {
                let _bid_runs = run_sheets(config, bid_sheets);
                run_out.add(format!("bid runs: (logging not implemented)\n"));
            } else {
                run_out.add(format!("sumbit aborted! bad bids",));
            }
        } else {
            run_out.add(format!("submit aborted! bad sim_bids",));
        }
    } else {
        run_out.add(format!("submit aborted! bad asks",));
    }
    run_out
}

fn mail_log(email: &str, order: &types::Order, run_log: &log::RunLog) {
    let subject = format!("{}", order.pair);
    let out = format!(
        "order #{} {} {:0.4} {:0.4}\n{}",
        order.id, order.pair, order.cost, order.profit, run_log
    );
    email::send(email, &subject, &out);
}

fn count_bad_sheets<M, N, O, P, T, S>(sheets: &Vec<(M, N, O, P, Vec<Result<T, S>>)>) -> usize {
    sheets.into_iter().fold(0, |memo, (m, n, o, p, s)| {
        memo + s.iter().filter(|sh| sh.is_err()).collect::<Vec<_>>().len()
    })
}

fn format_runs(
    runs: &Vec<(
        &config::Exchange,
        types::AskBid,
        types::Ticker,
        f64,
        Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
    )>,
) -> String {
    runs.iter().enumerate().fold(
        String::new(),
        |mut m, (idx, (exchange, askbid, token, total, sheets))| {
            let line = sheets
                .iter()
                .enumerate()
                .fold(String::new(), |mut m, (idx, r)| {
                    let part = match r {
                        Ok(sheet) => format!("{:?}", sheet),
                        Err(err) => err.to_string(),
                    };
                    let out = format!("offr #{}: {} {}", idx, exchange.settings.name, part);
                    m.push_str(&out);
                    m
                });
            m.push_str(&format!("{}: {}", exchange.settings.name, line));
            m
        },
    )
}

// rust to learn
#[derive(Clone, Copy)]
enum Mode {
    Simulate,
    Real,
}

fn build_books<'a>(
    config: &config::Config,
    wallet: &wallet::Wallet,
    books: &types::Books,
    exchanges: &'a config::ExchangeList,
    mode: Mode,
) -> Vec<(
    &'a config::Exchange,
    types::AskBid,
    types::Ticker,
    f64,
    Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
)> {
    books.books.iter().fold(Vec::new(), |mut memo, book| {
        let exchange_name = book.market.source.name.clone();
        let buy_token = match books.askbid {
            types::AskBid::Ask => &book.market.base,
            types::AskBid::Bid => &book.market.quote,
        };
        match exchanges.find_by_name(&exchange_name) {
            Some(exchange) => {
                let full = if exchange.settings.enabled {
                    let (total, sheets) =
                        build_book(config, wallet, &books.askbid, book, exchange, mode);
                    (
                        exchange,
                        books.askbid.clone(),
                        buy_token.clone(),
                        total,
                        sheets,
                    )
                } else {
                    (
                        exchange,
                        books.askbid.clone(),
                        buy_token.clone(),
                        0.0,
                        (vec![Err(exchange::ExchangeError::build_box(format!(
                            "exchange {} is disabled!",
                            exchange_name
                        )))]),
                    )
                };
                memo.push(full)
            }
            None => println!("exchange detail not found for: {:#?}", exchange_name),
        }
        memo
    })
}

fn build_book(
    config: &config::Config,
    wallet: &wallet::Wallet,
    askbid: &types::AskBid,
    book: &types::Book,
    exchange: &config::Exchange,
    mode: Mode,
) -> (
    f64,
    Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
) {
    let (total, sheets) =
        book.offers
            .iter()
            .fold((0.0, Vec::new()), |(mut total, mut sheets), offer| {
                let (askbid, market, offer) = unswap(askbid, &book.market, offer);
                let sell_token = match askbid {
                    types::AskBid::Ask => &book.market.quote,
                    types::AskBid::Bid => &book.market.base,
                };
                println!(
                    "** {} {} {} {} {} => {}{}",
                    match mode {
                        Mode::Real => "BUILD",
                        Mode::Simulate => "SIMBUILD",
                    },
                    askbid,
                    exchange.settings.name,
                    &book.market,
                    offer,
                    offer.cost(askbid),
                    sell_token
                );
                let sheet =
                    match build_offer(config, &askbid, &exchange, &offer, &market, wallet, mode) {
                        Ok(capped_offer) => {
                            let sheet = match mode {
                                Mode::Real => exchange.api.build(
                                    &config.wallet_private_key,
                                    &askbid,
                                    &exchange.settings,
                                    &market,
                                    &capped_offer,
                                ),
                                Mode::Simulate => Ok(exchange::OrderSheet::Placebo),
                            };
                            total += capped_offer.cost(askbid);
                            sheet
                        }
                        Err(e) => Err(e),
                    };
                sheets.push(sheet);
                (total, sheets)
            });
    (total, sheets)
}

fn build_offer(
    config: &config::Config,
    askbid: &types::AskBid,
    exchange: &config::Exchange,
    offer: &types::Offer,
    market: &exchange::Market,
    wallet: &wallet::Wallet,
    mode: Mode,
) -> Result<types::Offer, Box<dyn std::error::Error>> {
    println!("Building offer {} {}", exchange, offer);
    let pub_addr = eth::privkey_to_addr(&config.wallet_private_key);
    let sell_token = match askbid {
        types::AskBid::Ask => &market.quote,
        types::AskBid::Bid => &market.base,
    };

    // add premium
    let mut offer_quote_adjusted = offer.quote;
    if let Some(premium) = config.spread_premium {
        let adjustor = match askbid {
            types::AskBid::Ask => 1.0 + premium,
            types::AskBid::Bid => 1.0,
        };
        offer_quote_adjusted = offer_quote_adjusted * adjustor;
        println!(
            "offer {} spread premium {} adjusted by x{} to {}",
            offer.quote, premium, adjustor, offer_quote_adjusted
        );
    }
    let premium_offer = types::Offer {
        base_qty: offer.base_qty,
        quote: offer_quote_adjusted,
    };

    let mut amount_limits = vec![];
    let offer_cost = premium_offer.cost(*askbid);
    amount_limits.push(offer_cost);
    println!("added amount_limit of {} from offer_cost", offer_cost);

    let wallet_token_balance =
        match wallet.find_coin_by_source_symbol(&pub_addr, &sell_token.symbol) {
            Ok(coin) => {
                match mode {
                    Mode::Simulate => coin.base_total(), // simulate a full wallet
                    Mode::Real => coin.base_total(),
                }
            }
            Err(_e) => {
                let modeword = match mode {
                    Mode::Simulate => "WARNING",
                    Mode::Real => "ERROR",
                };
                let err = exchange::ExchangeError::build_box(format!(
                    "{}: {} balance unknown for {}",
                    modeword, sell_token, &pub_addr
                ));
                match mode {
                    Mode::Simulate => (),
                    Mode::Real => return Err(err), // early return
                };
                0.0
            }
        };

    if exchange.settings.has_balances {
        let exchange_token_balance =
            match wallet.find_coin_by_source_symbol(&market.source_name, &sell_token.symbol) {
                Ok(coin) => {
                    match mode {
                        Mode::Simulate => 0.0, // pretend its empty
                        Mode::Real => coin.base_total(),
                    }
                }
                Err(_e) => 0.0, // not found means 0.0
            };

        if exchange_token_balance < offer_cost {
            let insufficient_balance = offer_cost - exchange_token_balance;
            if wallet_token_balance > insufficient_balance {
                println!(
                    "Deposit: {:0.4} {} from wallet (offer_cost {:0.4} and exchage balance {:0.4})",
                    insufficient_balance, &sell_token.symbol, offer_cost, exchange_token_balance,
                );
                match mode {
                    Mode::Simulate => println!("Simulate deposit skipped"), // not a limitation in simulate
                    Mode::Real => exchange.api.deposit(
                        &config.wallet_private_key,
                        &exchange.settings,
                        insufficient_balance,
                        &sell_token,
                    ),
                }
            }
        } else {
            amount_limits.push(exchange_token_balance);
            println!(
                "added amount_limit of {}{} from {}  balance",
                exchange_token_balance, &sell_token.symbol, &market.source_name
            )
        }
    };

    amount_limits.push(wallet_token_balance);
    println!(
        "added amount_limit of {} from {} balance",
        wallet_token_balance, &pub_addr
    );

    // limit
    match wallet.find_coin_by_source_symbol("limit", &sell_token.symbol) {
        Ok(coin) => {
            let wallet_coin_limit = wallet.coin_limit(&sell_token.symbol);
            amount_limits.push(wallet_coin_limit);
            println!(
                "added amount_limit of {} from wallet_coin_limit",
                wallet_coin_limit
            );
        }
        Err(_e) => {
            let _err = exchange::ExchangeError::build_box(format!(
                "WARNING: {} wallet limit not set",
                sell_token
            ));
        }
    };

    let least_cost = eth::minimum(&amount_limits);
    println!("least_cost {} = min of {:?}", least_cost, &amount_limits);
    let least_qty = match askbid {
        types::AskBid::Ask => least_cost / premium_offer.quote,
        types::AskBid::Bid => least_cost,
    };
    if least_cost < offer_cost {
        exchange::ExchangeError::build_box(format!(
            "{} balance capped at {:0.4}. adj qty {:0.4}",
            sell_token, least_cost, least_qty
        ));
    }

    let least_quote = match askbid {
        types::AskBid::Ask => least_cost,
        types::AskBid::Bid => least_cost * premium_offer.quote,
    };
    let least_base = match askbid {
        types::AskBid::Ask => least_cost / premium_offer.quote,
        types::AskBid::Bid => least_cost,
    };

    let minimums = exchange.api.market_minimums(&market, &exchange.settings);
    match minimums {
        Some((base_minimum, quote_minimum)) => {
            println!(
                "{} market minimums {} base_minimum={:?} quote_minimum={:?}",
                exchange.settings.name, market, base_minimum, quote_minimum
            );
            match base_minimum {
                Some(minimum) => {
                    if minimum > least_base {
                        let err = exchange::ExchangeError::build_box(format!(
                            "{} base minimum {:0.4} NOT met with {:0.4}{}",
                            &market, minimum, least_base, &market.base
                        ));
                        return Err(err);
                    } else {
                        println!(
                            "{} base minimum {:0.4} met with {}{}",
                            &market, minimum, least_base, &market.base
                        );
                    }
                }
                None => (),
            };
            match quote_minimum {
                Some(minimum) => {
                    if minimum > least_quote {
                        let err = exchange::ExchangeError::build_box(format!(
                            "{} quote minimum of {:0.4} NOT met with {:0.4}{}",
                            &market, minimum, least_quote, &market.quote
                        ));
                        return Err(err);
                    } else {
                        println!(
                            "{} quote minimum {:0.4} met with {}{}",
                            &market, minimum, least_quote, &market.quote
                        );
                    }
                }
                None => (),
            };
        }
        None => {
            println!(
                "{} market minimums {} WARNING: no data",
                exchange.settings.name, market
            );
        }
    }

    let capped_offer = types::Offer {
        base_qty: least_qty,
        quote: premium_offer.quote,
    };
    Ok(capped_offer)
}

fn run_sheets(
    config: &config::Config,
    sheets: Vec<(
        &config::Exchange,
        types::AskBid,
        types::Ticker,
        f64,
        Vec<Result<exchange::OrderSheet, Box<dyn std::error::Error>>>,
    )>,
) {
    sheets
        .into_iter()
        .for_each(|(exchange, askbid, token, total, sheets)| {
            sheets.into_iter().for_each(|sheet_opt| match sheet_opt {
                Ok(sheet) => {
                    run_sheet(config, sheet, exchange);
                }
                Err(e) => (),
            });
            if exchange.settings.has_balances {
                sweep(&config.wallet_private_key, &exchange, &token);
            };
        });
}

fn sweep(
    private_key: &str,
    exchange: &config::Exchange,
    token: &types::Ticker,
) -> Option<Box<dyn std::error::Error>> {
    let my_addr = eth::privkey_to_addr(private_key);
    let direction = exchange::TransferDirection::Withdrawal;
    let balance_opt = exchange_balance(&my_addr, exchange, token);
    match balance_opt {
        Some(balance) => run_transfer(private_key, direction, exchange, balance, token),
        None => {
            println!(
                "no balance found for {}. skipping withdraw",
                exchange.settings.name
            );
            None
        }
    }
}

fn exchange_balance(
    public_key: &str,
    exchange: &config::Exchange,
    token: &types::Ticker,
) -> Option<f64> {
    let exchange_coins = exchange_coins(&public_key, exchange);
    let winner = exchange_coins
        .iter()
        .find(|c| c.ticker_symbol == token.symbol);
    match winner {
        Some(coin) => {
            let total = coin.base_total();
            println!("{} {} {}", exchange.settings.name, token, total);
            Some(total)
        }
        None => None,
    }
}

fn run_sheet(
    config: &config::Config,
    sheet: exchange::OrderSheet,
    exchange: &config::Exchange,
) -> Result<String, Box<dyn std::error::Error>> {
    println!("** RUN sheet {} {:?}", exchange, sheet);
    match exchange
        .api
        .submit(&config.wallet_private_key, &exchange.settings, sheet)
    {
        Ok(order_id) => {
            println!("* {} ORDER ID {}", exchange.settings.name, order_id);
            match wait_order(&exchange, &order_id) {
                exchange::OrderState::Filled => Ok(order_id),
                state => Err(exchange::ExchangeError::build_box(format!(
                    "transaction {:?}",
                    state
                ))),
            }
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

fn wait_order(exchange: &config::Exchange, order_id: &str) -> exchange::OrderState {
    let mut state = exchange::OrderState::Pending;
    let waiting_states = vec![exchange::OrderState::Pending, exchange::OrderState::Open];
    let mut repeat = true;
    while repeat {
        repeat = match waiting_states.iter().find(|&s| *s == state) {
            Some(_s) => {
                state = exchange.api.order_status(order_id, &exchange.settings);
                println!("{} {} => {:?}", exchange.settings.name, order_id, state);
                let delay = std::time::Duration::from_secs(3);
                std::thread::sleep(delay);
                true
            }
            None => false,
        }
    }
    state
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
        date: time::now_string(),
        pair: pair,
        cost: quantity * price,
        profit: 0.0,
        avg_price: price,
        ask_books: asks,
        bid_books: bids,
    }
}
