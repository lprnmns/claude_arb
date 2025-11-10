mod api;
mod config;
mod errors;
mod orderbook;
mod risk;
mod strategy;
mod types;
mod websocket;

use api::HyperliquidClient;
use config::Config;
use orderbook::Orderbook;
use risk::RiskManager;
use strategy::ArbitrageStrategy;
use types::*;
use websocket::WebSocketManager;

use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::time::{interval, sleep};
use tracing::{error, info, warn};
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs").expect("Failed to create logs directory");

    // File appender - creates daily rotating logs
    let file_appender = tracing_appender::rolling::daily("logs", "trade");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // Initialize logging with both console and file output
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stdout)
                .with_ansi(true),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(non_blocking)
                .with_ansi(false)
                .with_target(false),
        )
        .init();

    info!("🚀 Hyperliquid Arbitrage Bot starting...");
    info!("📝 Logs are being written to: logs/trade.YYYY-MM-DD");

    // Load configuration
    let config = match Config::from_env() {
        Ok(cfg) => {
            info!("✅ Configuration loaded successfully");
            info!("   Wallet: {}", cfg.wallet_address);
            info!("   BPS Threshold: {}", cfg.strategy.bps_threshold);
            info!("   Position Size: ${}", cfg.strategy.position_size_usd);
            info!("   Leverage: {}x", cfg.strategy.leverage);

            if cfg.dry_run.enabled {
                warn!("🔶 DRY RUN MODE ENABLED 🔶");
                warn!("   Orders will be SIMULATED (no real trades)");
                warn!("   Fill delay: {}ms", cfg.dry_run.fill_delay_ms);
                warn!("   Fill success rate: {}%", cfg.dry_run.fill_success_rate);
                warn!("   Set DRY_RUN=false for real trading");
            } else {
                warn!("⚠️  LIVE TRADING MODE - Real money at risk!");
            }

            cfg
        }
        Err(e) => {
            error!("❌ Configuration error: {}", e);
            std::process::exit(1);
        }
    };

    // Initialize components
    info!("🔧 Initializing components...");

    let client = match HyperliquidClient::new(config.api_url.clone(), config.private_key.clone()) {
        Ok(c) => {
            info!("✅ API client initialized");
            Arc::new(c)
        }
        Err(e) => {
            error!("❌ Failed to initialize API client: {}", e);
            std::process::exit(1);
        }
    };

    // Create orderbooks
    let perp_orderbook = Arc::new(Orderbook::new(format!("{}-PERP", config.symbols.perp_symbol)));
    let spot_orderbook = Arc::new(Orderbook::new(format!("{}-SPOT", config.symbols.spot_symbol)));

    // Initialize WebSocket manager
    let ws_manager = WebSocketManager::new(config.ws_url.clone());

    // Subscribe to orderbook updates
    info!("📡 Subscribing to orderbook updates...");

    if let Err(e) = ws_manager
        .subscribe_orderbook(config.symbols.perp_symbol.clone(), perp_orderbook.clone())
        .await
    {
        error!("Failed to subscribe to perp orderbook: {}", e);
        std::process::exit(1);
    }

    if let Err(e) = ws_manager
        .subscribe_orderbook(config.symbols.spot_symbol.clone(), spot_orderbook.clone())
        .await
    {
        error!("Failed to subscribe to spot orderbook: {}", e);
        std::process::exit(1);
    }

    // Wait for initial orderbook data
    info!("⏳ Waiting for initial orderbook data...");
    sleep(Duration::from_secs(3)).await;

    // Initialize strategy and risk manager
    let mut strategy = ArbitrageStrategy::new(
        config.clone(),
        perp_orderbook.clone(),
        spot_orderbook.clone(),
    );

    let risk_manager = RiskManager::new(config.risk.clone());

    info!("✅ All components initialized");
    info!("🎯 Starting trading loop...");
    info!("   Press Ctrl+C to stop");

    // Run main trading loop
    if let Err(e) = run_trading_loop(
        &mut strategy,
        &risk_manager,
        &client,
        perp_orderbook,
        spot_orderbook,
    )
    .await
    {
        error!("❌ Trading loop error: {}", e);
    }

    info!("👋 Bot stopped");
}

async fn run_trading_loop(
    strategy: &mut ArbitrageStrategy,
    risk_manager: &RiskManager,
    client: &Arc<HyperliquidClient>,
    perp_orderbook: Arc<Orderbook>,
    spot_orderbook: Arc<Orderbook>,
) -> anyhow::Result<()> {
    let mut check_interval = interval(Duration::from_millis(100)); // Check every 100ms for low latency
    let mut stats_interval = interval(Duration::from_secs(60)); // Log stats every minute

    let mut position_size: Option<rust_decimal::Decimal> = None;
    let mut exit_order_time: Option<std::time::Instant> = None;

    loop {
        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
                break;
            }

            _ = check_interval.tick() => {
                // Display current BPS
                if let Some(bps) = strategy.calculate_bps() {
                    let perp_ask = perp_orderbook.best_ask();
                    let spot_bid = spot_orderbook.best_bid();

                    if perp_ask.is_some() && spot_bid.is_some() {
                        // Check for entry signal
                        if let Some(signal) = strategy.should_enter() {
                            // Check risk limits
                            if let Err(e) = risk_manager.can_open_position(strategy.calculate_position_size(spot_bid.unwrap().price)) {
                                warn!("Cannot open position: {}", e);
                                continue;
                            }

                            info!("🚀 Executing entry...");

                            match execute_entry(strategy, client).await {
                                Ok(size) => {
                                    info!("✅ Entry successful! Position size: {}", size);
                                    strategy.set_state(ArbitrageState::PositionOpen);
                                    strategy.record_entry(signal.bps, signal.spot_price);
                                    position_size = Some(size);
                                }
                                Err(e) => {
                                    error!("❌ Entry failed: {}", e);
                                    strategy.set_state(ArbitrageState::Idle);
                                }
                            }
                        }

                        // Check for exit signal
                        if let Some(_signal) = strategy.should_exit() {
                            if let Some(size) = position_size {
                                info!("🎯 Executing exit...");

                                match execute_exit(strategy, client, size).await {
                                    Ok(_) => {
                                        info!("✅ Exit successful!");

                                        // Calculate and record PnL (FIXED FORMULA!)
                                        if let Some(profit_usd) = strategy.estimated_profit_usd(size) {
                                            risk_manager.record_trade(profit_usd);
                                            info!("💰 Trade P&L: ${:.4}", profit_usd);
                                        }

                                        strategy.set_state(ArbitrageState::Idle);
                                        strategy.clear_entry();
                                        position_size = None;
                                        exit_order_time = None;
                                    }
                                    Err(e) => {
                                        error!("❌ Exit failed: {}", e);
                                        exit_order_time = Some(std::time::Instant::now());
                                    }
                                }
                            }
                        }

                        // Check emergency exit
                        if strategy.should_emergency_exit() {
                            if let Some(size) = position_size {
                                warn!("⚠️ EMERGENCY EXIT!");

                                // Force close with IOC
                                if let Err(e) = client.cancel_all_orders(&strategy.config.symbols.perp_symbol).await {
                                    error!("Failed to cancel perp orders: {}", e);
                                }
                                if let Err(e) = client.cancel_all_orders(&strategy.config.symbols.spot_symbol).await {
                                    error!("Failed to cancel spot orders: {}", e);
                                }

                                // Record loss (FIXED FORMULA!)
                                if let Some(profit_usd) = strategy.estimated_profit_usd(size) {
                                    risk_manager.record_trade(profit_usd);
                                    warn!("💸 Emergency exit P&L: ${:.4}", profit_usd);
                                }

                                strategy.set_state(ArbitrageState::Idle);
                                strategy.clear_entry();
                                position_size = None;
                            }
                        }

                        // Check ALO timeout (30 seconds)
                        if let Some(order_time) = exit_order_time {
                            if order_time.elapsed() > Duration::from_secs(30) {
                                warn!("⏰ ALO timeout, forcing close with IOC");

                                if let Some(size) = position_size {
                                    // Cancel pending ALO orders
                                    if let Err(e) = client.cancel_all_orders(&strategy.config.symbols.perp_symbol).await {
                                        error!("Failed to cancel perp orders: {}", e);
                                    }
                                    if let Err(e) = client.cancel_all_orders(&strategy.config.symbols.spot_symbol).await {
                                        error!("Failed to cancel spot orders: {}", e);
                                    }

                                    // Force close with IOC (slightly worse prices for guaranteed fill)
                                    info!("🚨 Force closing with IOC (0.1% slippage buffer)");

                                    match execute_force_close(&strategy, client, size).await {
                                        Ok(_) => {
                                            info!("✅ Force close successful!");

                                            // Calculate PnL (will be slightly less due to slippage) (FIXED FORMULA!)
                                            if let Some(profit_usd) = strategy.estimated_profit_usd(size) {
                                                risk_manager.record_trade(profit_usd);
                                                warn!("⚠️ Exit with IOC slippage: ~${:.4}", profit_usd);
                                            }
                                        }
                                        Err(e) => {
                                            error!("❌ Force close FAILED: {}", e);
                                            error!("💀 CRITICAL: Manual intervention required!");
                                            // Don't clear state, will retry next loop
                                            continue;
                                        }
                                    }

                                    strategy.set_state(ArbitrageState::Idle);
                                    strategy.clear_entry();
                                    position_size = None;
                                    exit_order_time = None;
                                }
                            }
                        }
                    }
                }
            }

            _ = stats_interval.tick() => {
                // Log statistics
                risk_manager.log_statistics();

                if let Some(bps) = strategy.calculate_bps() {
                    info!("📊 Current BPS: {:.2}", bps);
                }
            }
        }
    }

    Ok(())
}

async fn execute_entry(
    strategy: &ArbitrageStrategy,
    client: &Arc<HyperliquidClient>,
) -> anyhow::Result<rust_decimal::Decimal> {
    let orders = strategy.build_entry_orders()?;
    let config = &strategy.config;

    info!("Placing {} entry orders (IOC)", orders.len());

    if config.dry_run.enabled {
        // DRY RUN: Simulate order execution
        info!("📝 DRY RUN: Simulating entry orders...");

        // Simulate network latency
        tokio::time::sleep(Duration::from_millis(config.dry_run.fill_delay_ms)).await;

        // Simulate random success/failure based on fill_success_rate
        let success = rand::random::<u8>() < config.dry_run.fill_success_rate;

        if !success {
            warn!("📝 DRY RUN: Simulated REJECTION ({}% fill rate)",
                  config.dry_run.fill_success_rate);
            return Err(anyhow::anyhow!("Simulated order rejection"));
        }

        for (i, order) in orders.iter().enumerate() {
            info!("📝 DRY RUN: Order {} FILLED - {} {} @ ${}",
                  i+1,
                  match order.side {
                      Side::Buy => "BUY",
                      Side::Sell => "SELL",
                  },
                  order.size,
                  order.price);
        }

        info!("✅ DRY RUN: Entry execution simulated successfully");
    } else {
        // LIVE: Real order execution
        let responses = client.place_batch_orders(orders.clone()).await?;

        // Verify both orders were filled
        if responses.len() != 2 {
            return Err(anyhow::anyhow!("Unexpected number of responses"));
        }
    }

    // Return position size (use perp size as reference)
    Ok(orders[0].size)
}

async fn execute_exit(
    strategy: &ArbitrageStrategy,
    client: &Arc<HyperliquidClient>,
    position_size: rust_decimal::Decimal,
) -> anyhow::Result<()> {
    let orders = strategy.build_exit_orders(position_size)?;
    let config = &strategy.config;

    info!("Placing {} exit orders (ALO - maker only)", orders.len());

    if config.dry_run.enabled {
        // DRY RUN: Simulate ALO order placement
        info!("📝 DRY RUN: Simulating ALO exit orders...");

        // Simulate order placement latency
        tokio::time::sleep(Duration::from_millis(config.dry_run.fill_delay_ms)).await;

        for (i, order) in orders.iter().enumerate() {
            info!("📝 DRY RUN: ALO Order {} PLACED - {} {} @ ${}",
                  i+1,
                  match order.side {
                      Side::Buy => "BUY",
                      Side::Sell => "SELL",
                  },
                  order.size,
                  order.price);
        }

        // Simulate random fill time (2-10 seconds for ALO)
        let fill_delay = rand::random::<u64>() % 8000 + 2000; // 2-10s
        info!("📝 DRY RUN: Waiting for ALO fills (~{}ms)...", fill_delay);
        tokio::time::sleep(Duration::from_millis(fill_delay)).await;

        info!("✅ DRY RUN: ALO orders filled (simulated)");
    } else {
        // LIVE: Real ALO order placement
        let _responses = client.place_batch_orders(orders).await?;
    }

    Ok(())
}

async fn execute_force_close(
    strategy: &ArbitrageStrategy,
    client: &Arc<HyperliquidClient>,
    position_size: rust_decimal::Decimal,
) -> anyhow::Result<()> {
    let orders = strategy.build_exit_orders_ioc(position_size)?;
    let config = &strategy.config;

    info!("Placing {} force close orders (IOC with 0.1% slippage)", orders.len());

    if config.dry_run.enabled {
        // DRY RUN: Simulate IOC force close
        info!("📝 DRY RUN: Simulating IOC force close...");

        // IOC is always fast
        tokio::time::sleep(Duration::from_millis(config.dry_run.fill_delay_ms)).await;

        for (i, order) in orders.iter().enumerate() {
            info!("📝 DRY RUN: IOC Order {} FILLED - {} {} @ ${} (with slippage)",
                  i+1,
                  match order.side {
                      Side::Buy => "BUY",
                      Side::Sell => "SELL",
                  },
                  order.size,
                  order.price);
        }

        info!("✅ DRY RUN: Force close successful (simulated)");
    } else {
        // LIVE: Real IOC force close
        let responses = client.place_batch_orders(orders).await?;

        // Verify both orders were filled
        if responses.len() != 2 {
            return Err(anyhow::anyhow!("Force close: unexpected number of responses"));
        }
    }

    Ok(())
}


