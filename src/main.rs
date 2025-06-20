use clap;

use yith::config;
use yith::email;
use yith::erc20;
use yith::errors;
use yith::eth;
use yith::etherscan;
use yith::exchange;
use yith::geth;
use yith::log;
use yith::redis;
use yith::time;
use yith::types;
use yith::wallet;
use yith::weth;

fn main() {
    let options_yaml = clap::load_yaml!("cli.yaml"); // load/parse at compile time
    let options = clap::App::from_yaml(options_yaml).get_matches();
    log::init();

    let config_filename = options.value_of("config").unwrap_or(config::FILENAME);
    let config: config::Config = config::read_type(config_filename);

    let wallet_filename = "wallet.yaml";
    let wallet: wallet::Wallet = config::read_type(wallet_filename);

    log::info!(
        "Yith {:#?} {} {}",
        config_filename,
        time::now_string(),
        if config.trade_live { "LIVE" } else { "DEMO" }
    );

    let exchanges_filename = "exchanges.yaml";
    let exchanges = config::hydrate_exchanges(exchanges_filename, &config)
        .unwrap_or_else(|c| panic!("{} {}", exchanges_filename, c));

    let etherscan = etherscan::Etherscan::new(&config.etherscan_key);
    config::ETHERSCAN.set(etherscan).unwrap(); // set-once global
    config::CONFIG.set(config).unwrap(); // set-once global

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
        let geth = geth::Client::build(&config.geth_url);
        let amount_str = exchange::quantity_in_base_units(amount, 18, 18).to_string();
        match action {
            "wrap" => {
                match weth::Weth::wrap(
                    geth,
                    &config.wallet_private_key,
                    weth::Direction::Wrap,
                    &amount_str,
                ) {
                    Ok(_yn) => None,
                    Err(e) => Some(e),
                }
            }
            "unwrap" => {
                match weth::Weth::wrap(
                    geth,
                    &config.wallet_private_key,
                    weth::Direction::Unwrap,
                    &amount_str,
                ) {
                    Ok(_yn) => None,
                    Err(e) => Some(e),
                }
            }
            _ => None,
        }
    } else if let Some(matches) = opts.subcommand_matches("erc20") {
        let action = matches.value_of("action").unwrap();
        let token = matches.value_of("token").unwrap();
        let exchange_name = matches.value_of("exchange").unwrap();
        let exchange = exchanges.find_by_name(exchange_name).unwrap();
        let geth = geth::Client::build(&config.geth_url);
        match action {
            "allowance" => {
                let allowance = erc20::Erc20::allowance(
                    geth,
                    &config.wallet_private_key,
                    &token,
                    exchange.settings.contract_address.as_ref().unwrap(),
                )
                .unwrap();
                println!("erc20 {} {} {}", token, exchange_name, allowance);
            }
            "approve" => {
                let approve = erc20::Erc20::approve(
                    geth,
                    &config.wallet_private_key,
                    &token,
                    exchange.settings.contract_address.as_ref().unwrap(),
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
                )));
            }
        };
        //let direction = matches.value_of("direction").unwrap().into();
        let amount = matches.value_of("amount").unwrap();
        let symbol = matches.value_of("token").unwrap();
        let exchange_name = matches.value_of("exchange").unwrap();
        let exchange = exchanges.find_by_name(exchange_name).unwrap();

        if amount == "sweep" {
            sweep(&config.wallet_private_key, &exchange, &symbol.into())
        } else {
            match run_transfer(
                &config.wallet_private_key,
                direction,
                &exchange,
                amount.parse::<f64>().unwrap(),
                &symbol.into(),
            ) {
                Ok(_tx) => None,
                Err(e) => Some(e),
            }
        }
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

        // final balances
        wallet.reset();
        scan_wallet(&mut wallet.coins, &exchanges);
        wallet.print_with_price();

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
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let config = config::CONFIG.get().unwrap();
    println!(
        "{:?} into {} {:0.5} {}",
        direction, exchange.settings.name, amount, token
    );
    let public_addr = eth::privkey_to_addr(private_key);
    let escan = config::ETHERSCAN.get().unwrap();
    let etoken = escan.tokens.get(token).unwrap();
    let start_wallet = etherscan_coin(
        &public_addr,
        &etoken.symbol,
        &etoken.hash,
        &config.etherscan_key,
    );
    println!(
        "run_transfer starting wallet balance {:0.5} {}",
        start_wallet, token
    );
    let start_exchange = match exchange_balance(&public_addr, exchange, token) {
        Some(balance) => balance,
        None => 0.0,
    };
    println!(
        "run_transfer {} starting exchange balance {:0.5} {}",
        exchange.settings.name, start_exchange, token
    );
    let tid_opt = match direction {
        exchange::TransferDirection::Withdraw => {
            exchange
                .api
                .withdraw(private_key, &exchange.settings, amount, token)
        }
        exchange::TransferDirection::Deposit => {
            exchange
                .api
                .deposit(private_key, &exchange.settings, amount, token)
        }
    };
    match tid_opt {
        Ok(tid) => match tid {
            Some(tferid) => match wait_transfer(&tferid, &public_addr, exchange) {
                exchange::BalanceStatus::Complete => {
                    let stop_exchange = match exchange_balance(&public_addr, exchange, token) {
                        Some(balance) => balance,
                        None => 0.0,
                    };
                    println!(
                        "run_transfer {} stop exchange balance {:0.5} {}",
                        exchange.settings.name, stop_exchange, token
                    );
                    let stop_wallet = etherscan_coin(
                        &public_addr,
                        &etoken.symbol,
                        &etoken.hash,
                        &config.etherscan_key,
                    );
                    println!(
                        "run_transfer stop wallet balance {:0.5} {}",
                        stop_wallet, token
                    );
                    let exchange_change = match direction {
                        exchange::TransferDirection::Withdraw => start_exchange - stop_exchange,
                        exchange::TransferDirection::Deposit => stop_exchange - start_exchange,
                    };
                    let wallet_change = match direction {
                        exchange::TransferDirection::Withdraw => stop_wallet - start_wallet,
                        exchange::TransferDirection::Deposit => start_wallet - stop_wallet,
                    };
                    let exchange_diff = amount - exchange_change;
                    let wallet_diff = amount - wallet_change;
                    println!(
                        "run_transfer {} actual exchange change {:0.5} fee {:0.5} (missing from amount {})",
                        token, exchange_change, exchange_diff, amount
                    );
                    println!(
                        "run_transfer {} actual wallet change {:0.5} fee {:0.5} (missing from amount {})",
                        token, wallet_change, wallet_diff, amount
                    );
                    Ok(None)
                }
                exchange::BalanceStatus::InProgress => Err(exchange::ExchangeError::build_box(
                    "transfer status weird timeout".to_string(),
                )),
                exchange::BalanceStatus::Error => Err(exchange::ExchangeError::build_box(
                    "transfer status is error!".to_string(),
                )),
            },
            None => Ok(Some(
                "skipped balance wait due to missing transfer id".to_string(),
            )),
        },
        Err(e) => Err(e),
    }
}

fn wait_transfer(
    transfer_id: &str,
    public_addr: &str,
    exchange: &config::Exchange,
) -> exchange::BalanceStatus {
    println!("wait_transfer watching {}", transfer_id);
    let start = time::now();
    let mut status = exchange::BalanceStatus::InProgress;
    let mut done = false;
    while !done {
        let waited = start.elapsed();
        status = exchange
            .api
            .transfer_status(transfer_id, public_addr, &exchange.settings);
        println!(
            "wait_transfer {} {:?} {}",
            transfer_id,
            status,
            time::duration_words(waited)
        );
        done = match status {
            exchange::BalanceStatus::Complete => true,
            exchange::BalanceStatus::InProgress => {
                time::sleep(10000);
                false
            }
            exchange::BalanceStatus::Error => true,
        };
    }
    status
}

fn scan_wallet(coins: &mut Vec<wallet::WalletCoin>, exchanges: &config::ExchangeList) {
    let config = config::CONFIG.get().unwrap();
    let my_addr = eth::privkey_to_addr(&config.wallet_private_key);
    print!("etherscan BALANCES for ");
    let mut eth_coins = etherscan_coins(&my_addr, coins, &config.etherscan_key);
    println!();
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

fn mail_log(email: &str, order: &types::Order, run_log: &log::RunLog) {
    let subject = format!("{}", order.pair);
    let out = format!(
        "order #{} {} {:0.4} {:0.4}\n{}",
        order.id, order.pair, order.cost, order.profit, run_log
    );
    email::send(email, &subject, &out);
}

fn count_good_total<M, N, O, T, S>(booksheets: &Vec<(M, N, O, f64, Vec<Result<T, S>>)>) -> f64 {
    booksheets
        .into_iter()
        .fold(0.0, |memo, (_m, _n, _o, total, _sheets)| memo + total)
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
    let ask_sheets_good_total = count_good_total(&ask_sheets);

    if order.ask_books.cost_total() == 0.0 || ask_sheets_good_total > 0.0 {
        let sim_bid_sheets =
            build_books(config, wallet, &order.bid_books, exchanges, Mode::Simulate);
        let sim_bid_sheets_good_total = count_good_total(&sim_bid_sheets);

        if order.bid_books.cost_total() == 0.0 || sim_bid_sheets_good_total > 0.0 {
            let _ask_runs = run_sheets(config, ask_sheets);

            // wallet refresh
            println!("=wallet refresh");
            wallet.reset();
            scan_wallet(&mut wallet.coins, &exchanges);

            let bid_sheets = build_books(config, wallet, &order.bid_books, exchanges, Mode::Real);
            let bid_sheets_good_total = count_good_total(&bid_sheets);

            if bid_sheets_good_total > 0.0 {
                let _bid_runs = run_sheets(config, bid_sheets);
            } else {
                run_out.add(format!(
                    "sumbit aborted! {} good total bids",
                    bid_sheets_good_total
                ));
            }
        } else {
            run_out.add(format!(
                "submit aborted! {} good total sim_bids",
                sim_bid_sheets_good_total
            ));
        }
    } else {
        run_out.add(format!(
            "submit aborted! {} good total asks",
            ask_sheets_good_total
        ));
    }
    run_out
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
                println!("->{} sheets {:?}", exchange_name, full.4);
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
    let sell_token = match askbid {
        types::AskBid::Ask => &book.market.quote,
        types::AskBid::Bid => &book.market.base,
    };
    println!(
        "** {} {} {} sell_token: {}",
        match mode {
            Mode::Real => "BOOK",
            Mode::Simulate => "SIMBOOK",
        },
        askbid,
        &book.market,
        sell_token
    );
    let pub_addr = eth::privkey_to_addr(&config.wallet_private_key);
    let mut wallet_token_balance =
        match wallet.find_coin_by_source_symbol(&pub_addr, &sell_token.symbol) {
            Ok(coin) => {
                let wallet_pre_dust = match mode {
                    Mode::Simulate => book.cost_total(askbid.clone()), // simulate a full wallet
                    Mode::Real => coin.base_total(),
                };
                if sell_token.symbol == "ETH" {
                    let wallet_post_dust = if wallet_pre_dust > config.eth_dust {
                        let subtotal = wallet_pre_dust - config.eth_dust;
                        println!(
                            "wallet balance {} {} - {} dust min = {}",
                            wallet_pre_dust, sell_token.symbol, config.eth_dust, subtotal
                        );
                        subtotal
                    } else {
                        println!(
                            "wallet balance {} {} below {} dust min. skip.",
                            wallet_pre_dust, sell_token.symbol, config.eth_dust
                        );
                        0.0
                    };
                    wallet_post_dust
                } else {
                    wallet_pre_dust
                }
            }
            Err(_e) => {
                let modeword = match mode {
                    Mode::Simulate => "WARNING",
                    Mode::Real => "ERROR",
                };
                match mode {
                    Mode::Simulate => (),
                    Mode::Real => {
                        panic!(
                            "{}: no balance available for {} (in {}). panicing.",
                            modeword, sell_token, &pub_addr
                        );
                    }
                };
                0.0
            }
        };

    let mut exchange_balance = None;
    if exchange.settings.has_balances {
        let exchange_token_balance =
            match wallet.find_coin_by_source_symbol(&book.market.source.name, &sell_token.symbol) {
                Ok(coin) => {
                    match mode {
                        Mode::Simulate => book.cost_total(askbid.clone()), // pretend its full
                        Mode::Real => coin.base_total(),
                    }
                }
                Err(_e) => 0.0, // not found means 0.0
            };
        exchange_balance = Some(exchange_token_balance);
        println!(
            "wallet balance {} {} enhanced by {} balance {} {}",
            wallet_token_balance,
            &sell_token.symbol,
            &book.market.source.name,
            exchange_token_balance,
            &sell_token.symbol
        );
        wallet_token_balance += exchange_token_balance;
    }

    let rollup_offer = book.offers.iter().fold(
        types::Offer {
            base_qty: 0.0,
            quote: 0.0,
        },
        |mut rolled, offer| {
            rolled.base_qty += offer.base_qty;
            rolled.quote = offer.quote;
            println!("rollup {} added {}", rolled, offer);
            rolled
        },
    );
    let (total, processed_offers) =
        vec![rollup_offer]
            .iter()
            .fold((0.0, Vec::new()), |(mut total, mut offers), offer| {
                let (askbid, market, offer) = unswap(askbid, &book.market, offer);
                println!(
                    "** {} {} {} {} => {}{}",
                    match mode {
                        Mode::Real => "BUILD",
                        Mode::Simulate => "SIMBUILD",
                    },
                    askbid,
                    &book.market,
                    offer,
                    offer.cost(askbid),
                    sell_token,
                );
                let capped_offer_opt = match build_offer(
                    config,
                    &askbid,
                    &exchange,
                    &offer,
                    &market,
                    wallet_token_balance - total,
                    wallet,
                ) {
                    Ok(capped_offer) => {
                        let value = capped_offer.cost(askbid);
                        if value > 0_f64 {
                            total += value;
                            Ok((capped_offer, market))
                        } else {
                            Err(errors::MainError::build_box(format!(
                                "skipping zero value transaction"
                            )))
                        }
                    }
                    Err(e) => Err(e),
                };
                offers.push(capped_offer_opt);
                (total, offers)
            });
    println!("{} processed_offers done", processed_offers.len());
    if let Some(exchange_token_balance) = exchange_balance {
        if total > exchange_token_balance {
            println!(
                "order total {} exceeds exchange balance {}",
                total, exchange_token_balance
            );
            let missing = total - exchange_token_balance;
            println!(
                "Deposit: {:0.4}{} from wallet (offer_cost {:0.4})",
                missing, &sell_token.symbol, total
            );
            match mode {
                Mode::Simulate => println!("Simulate deposit skipped"), // not a limitation in simulate
                Mode::Real => {
                    let direction = exchange::TransferDirection::Deposit;
                    let _deposit_id = run_transfer(
                        &config.wallet_private_key,
                        direction,
                        &exchange,
                        missing,
                        &sell_token,
                    );
                }
            }
        } else {
            println!(
                "order total {:0.5} is met by exchange balance {:0.5}. no despoit necessary.",
                total, exchange_token_balance
            );
        }
    }
    println!("submitting {} processed_offers", processed_offers.len());
    let sheets = processed_offers
        .into_iter()
        .map(|offer_opt| match offer_opt {
            Ok((capped_offer, market)) => match mode {
                Mode::Real => exchange.api.build(
                    &config.wallet_private_key,
                    &askbid,
                    &exchange.settings,
                    &market,
                    &capped_offer,
                ),
                Mode::Simulate => Ok(exchange::OrderSheet::Placebo),
            },
            Err(e) => Err(e),
        })
        .collect();
    (total, sheets)
}

fn build_offer(
    config: &config::Config,
    askbid: &types::AskBid,
    exchange: &config::Exchange,
    offer: &types::Offer,
    market: &exchange::Market,
    wallet_token_balance: f64,
    wallet: &wallet::Wallet,
) -> Result<types::Offer, Box<dyn std::error::Error>> {
    println!("Building offer {} {}", exchange, offer);
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
            "quote {}{} spread premium {} adjusted by x{} to {}",
            offer.quote, &market.quote, premium, adjustor, offer_quote_adjusted
        );
    }
    let premium_offer = types::Offer {
        base_qty: offer.base_qty,
        quote: offer_quote_adjusted,
    };

    let mut amount_limits = vec![];
    let offer_cost = premium_offer.cost(*askbid);
    amount_limits.push(offer_cost);
    println!("added amount_limit of {:0.5} from offer_cost", offer_cost);

    amount_limits.push(wallet_token_balance);
    println!(
        "added amount_limit of {:0.5} from wallet balance",
        wallet_token_balance
    );

    // limit
    match wallet.find_coin_by_source_symbol("limit", &sell_token.symbol) {
        Ok(_coin) => {
            let wallet_coin_limit = wallet.coin_limit(&sell_token.symbol);
            amount_limits.push(wallet_coin_limit);
            println!(
                "added amount_limit of {:0.5} from wallet_coin_limit",
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
    println!(
        "least_cost {:0.5} = min of {:?}",
        least_cost, &amount_limits
    );
    let least_qty = match askbid {
        types::AskBid::Ask => least_cost / premium_offer.quote,
        types::AskBid::Bid => least_cost,
    };
    if least_cost < offer_cost {
        println!(
            "{} balance capped at {:0.5}. adj qty {:0.5}",
            sell_token, least_cost, least_qty
        );
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
            sheets.into_iter().for_each(|sheet_opt| {
                let _quiet = match sheet_opt {
                    Ok(sheet) => run_sheet(config, sheet, exchange),
                    Err(_e) => {
                        println!(
                            "order_sheet skipped {} {} {}",
                            exchange.settings.name, askbid, token
                        );
                        Ok("-skipped-".to_string())
                    }
                };
            });
            if exchange.settings.has_balances {
                if total > 0.0 {
                    sweep(&config.wallet_private_key, &exchange, &token);
                }
            };
        });
}

fn sweep(
    private_key: &str,
    exchange: &config::Exchange,
    token: &types::Ticker,
) -> Option<Box<dyn std::error::Error>> {
    println!("** Sweep {} {}", exchange.settings.name, token);
    let my_addr = eth::privkey_to_addr(private_key);
    let direction = exchange::TransferDirection::Withdraw;
    let balance_opt = exchange_balance(&my_addr, exchange, token);
    match balance_opt {
        Some(balance) => match run_transfer(private_key, direction, exchange, balance, token) {
            Ok(_tid) => None,
            Err(e) => Some(e),
        },
        None => {
            println!(
                "no balance found for {}. skipping withdraw/sweep",
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
            println!("{} balance {} {}", exchange.settings.name, token, total);
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
    println!("** RUN sheet {}", exchange);
    let submit_opt = if config.trade_live {
        exchange
            .api
            .submit(&config.wallet_private_key, &exchange.settings, sheet)
    } else {
        println!("=DEMO mode no submit placeholder-order-id");
        Ok("placeholder-order-id".to_string())
    };
    match submit_opt {
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
        println!("{} balance check for 0x{}", exchange.settings.name, my_addr);
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
    let mut coins = Vec::<wallet::WalletCoin>::new();
    for coin in wallet_coins {
        print!("{} ", coin.ticker_symbol);
        let balance = etherscan_coin(my_addr, &coin.ticker_symbol, &coin.contract, api_key);
        let eth_coin =
            wallet::WalletCoin::build(&coin.ticker_symbol, &coin.contract, &my_addr, balance);
        coins.push(eth_coin);
    }
    coins
}

fn etherscan_coin(my_addr: &str, symbol: &str, contract_addr: &str, api_key: &str) -> f64 {
    let balance = etherscan::balance(my_addr, contract_addr, api_key);
    let token = types::Ticker {
        symbol: symbol.to_string(),
    };
    let escan = config::ETHERSCAN.get().unwrap();
    let decimals = match escan.tokens.get(&token) {
        Some(token_detail) => token_detail.decimals,
        None => 0,
    };
    eth::wei_to_eth(balance, decimals)
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
    let swapped = matches.is_present("swapped");
    println!(
        "** MANUAL {} {} {}{}@{}{}{}",
        exchange,
        side,
        quantity,
        base_symbol,
        price,
        quote_symbol,
        if swapped { " SWAPPED" } else { "" }
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
        swapped: swapped,
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
        swapped: swapped,
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
        id: "#manual-id".to_string(),
        date: time::now_string(),
        pair: pair,
        cost: quantity * price,
        trade_profit: 0.0,
        profit: 0.0,
        fee_network: 0.0,
        quote_usd: 0.0,
        network_usd: 0.0,
        ask_books: asks,
        bid_books: bids,
    }
}
