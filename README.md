# Hyperliquid Arbitrage Bot

High-performance arbitrage bot for trading perp-spot spread on Hyperliquid DEX.

## Features

- ⚡ **Low Latency**: Written in Rust with async tokio runtime
- 📊 **Real-time Orderbook**: WebSocket-based L2 orderbook updates
- 🎯 **BPS-based Strategy**: Enter when spread > threshold, exit when spread narrows
- 🛡️ **Risk Management**: Position limits, daily loss limits, emergency stop
- 📈 **Performance Tracking**: Win rate, P&L tracking, trade statistics
- 🔄 **Smart Order Execution**: IOC for entry, ALO for exit (maker rebate)

## Architecture

### Core Components

1. **Config Module** (`src/config.rs`)
   - Environment variable loading
   - Strategy and risk parameters
   - Wallet configuration

2. **API Client** (`src/api.rs`)
   - Hyperliquid HTTP API integration
   - EIP-712 transaction signing
   - Batch order execution
   - Nonce-based order cancellation

3. **WebSocket Manager** (`src/websocket.rs`)
   - Real-time L2 orderbook updates
   - Automatic reconnection
   - Multi-symbol support

4. **Orderbook** (`src/orderbook.rs`)
   - In-memory L2 orderbook
   - Lock-free updates with RwLock
   - BPS and liquidity calculations

5. **Strategy Engine** (`src/strategy.rs`)
   - Perp-spot spread arbitrage logic
   - Entry/exit signal generation
   - Position sizing
   - Emergency exit conditions

6. **Risk Manager** (`src/risk.rs`)
   - Position size limits
   - Daily P&L tracking
   - Win/loss statistics
   - Emergency stop functionality

## Strategy Logic

### Entry Conditions
- Spread (perp - spot) exceeds BPS threshold
- Sufficient liquidity available
- Risk limits not exceeded
- Uses **IOC orders** for immediate execution

### Exit Conditions
- Spread narrows to 0 or negative
- Emergency: Spread widens beyond max loss threshold
- Uses **ALO orders** (maker only) for fee rebate
- Timeout: Force close with IOC after 30 seconds

### Position Structure
- **Perp**: Short with 2-3x leverage
- **Spot**: Long with 1x (no leverage)
- **Sizing**: Based on USD position size and leverage

## Testing Without Capital (DRY-RUN Mode)

⭐ **Test the bot without risking real money!**

```bash
# 1. Enable dry-run mode in .env
DRY_RUN=true

# 2. Run the bot
cargo run

# 3. Watch simulated trades with real market data
# - Real WebSocket orderbook updates
# - Real BPS calculations
# - Simulated order execution
# - Full P&L tracking
```

See [DRY_RUN_GUIDE.md](DRY_RUN_GUIDE.md) for detailed testing instructions.

---

## Configuration

Edit `.env` file:

```bash
# API Configuration
HYPERLIQUID_API_URL=https://api.hyperliquid.xyz
HYPERLIQUID_WS_URL=wss://api.hyperliquid.xyz/ws

# Your private key (KEEP SECRET!)
PRIVATE_KEY=your_private_key_here

# Strategy Parameters
BPS_THRESHOLD=20.0          # Enter when spread > 20 bps
POSITION_SIZE_USD=150.0     # Position size in USD
LEVERAGE=2                  # 2x or 3x recommended
TIMEOUT_SECONDS=30          # ALO timeout before force close
MAX_ORDERBOOK_AGE_MS=1500   # Ignore spreads if either book is older than 1.5s

# Risk Management
MAX_LOSS_BPS=50.0           # Emergency exit if loss > 50 bps
DAILY_LOSS_LIMIT_USD=20.0   # Stop trading if daily loss exceeds limit
EMERGENCY_STOP=false        # Manual emergency stop

# Trading Pair
PERP_SYMBOL=HYPE
SPOT_SYMBOL=HYPE

# Logging
RUST_LOG=info
```

## Installation

### Prerequisites
- Rust 1.70+ (`cargo --version`)
- 2 CPU cores, 16GB RAM recommended
- Low-latency internet connection

### Setup

1. Clone the repository
```bash
cd /path/to/claude_arb
```

2. Copy environment file
```bash
cp .env.example .env
# Edit .env with your private key
```

3. Build (release mode for production)
```bash
cargo build --release
```

4. Run
```bash
# Development
cargo run

# Production (optimized)
./target/release/hyperliquid-arb-bot
```

## Testing

### Unit Tests
```bash
cargo test
```

### Testnet/Paper Trading
Start with small capital (10-50 USDC) on mainnet to test:
```bash
POSITION_SIZE_USD=10.0 cargo run
```

Monitor for:
- Order fill rates
- Latency (should be <100ms for entry)
- P&L per trade
- Risk manager alerts

### Production Checklist
- [ ] Test with 10 USDC for 24 hours
- [ ] Verify order execution (no rejections)
- [ ] Check WebSocket stability (no disconnects)
- [ ] Monitor funding rates (affects perp position)
- [ ] Confirm maker rebates on exits

## Performance Optimization

### Network
- Deploy close to Hyperliquid servers (Mumbai, India recommended)
- Use dedicated RPC node or run your own Hyperliquid node
- Enable TCP fast open: `net.ipv4.tcp_fastopen=3`

### Application
- Release build with LTO and optimization
- Check interval: 100ms (10 checks/second)
- Multi-core: 2 threads (Tokio work-stealing)

### Expected Metrics
- **Latency**: <50ms API round-trip
- **BPS checks**: 100-500/second
- **Order execution**: <200ms entry, <1s exit (ALO)
- **Rejected orders**: <1%

## Monitoring

### Logs
```bash
# Real-time monitoring
tail -f bot.log

# Filter errors
grep ERROR bot.log

# Trade statistics
grep "Risk Statistics" bot.log
```

### Key Metrics
- **Daily P&L**: Track cumulative profit
- **Win Rate**: Should be >60% for profitable operation
- **Avg Win vs Avg Loss**: Profit factor
- **Current BPS**: Monitor spread opportunities

## Risk Warnings

⚠️ **IMPORTANT**
- Start with small capital (100-300 USDC)
- Use 2x leverage maximum (3x increases liquidation risk)
- Monitor funding rates (8-hour intervals)
- Check spot liquidity before increasing size
- Set appropriate daily loss limits

### Known Risks
1. **Funding Rate**: Negative funding costs money on short perp
2. **Spread Widening**: Emergency exit triggers if spread increases
3. **Liquidity**: Spot may have less depth than perp
4. **Slippage**: IOC orders may get partial fills
5. **API Latency**: Network delays can miss opportunities

## Production Deployment

### Mumbai VPS Setup
```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone and build
git clone <repo_url>
cd hyperliquid-arb-bot
cargo build --release

# 3. Setup systemd service (optional)
sudo cp hyperliquid-arb.service /etc/systemd/system/
sudo systemctl enable hyperliquid-arb
sudo systemctl start hyperliquid-arb

# 4. Monitor
sudo journalctl -u hyperliquid-arb -f
```

### Running Your Own Node
```bash
# For lowest latency, run local Hyperliquid node
docker run -d --name hyperliquid-node \\
  --disable-output-file-buffering \\
  -p 26657:26657 hyperliquid/node

# Update .env
HYPERLIQUID_API_URL=http://localhost:26657
```

## Troubleshooting

### Orders Rejected
- Check rate limits (batch orders help)
- Verify account has sufficient margin
- Confirm orderbook has liquidity

### WebSocket Disconnects
- Check internet stability
- Automatic reconnection built-in
- Consider backup RPC providers

### No Entry Signals
- BPS threshold may be too high (try 15-18 bps)
- Check if spread is too tight
- Verify orderbook data is updating

### Losses
- Spread may be widening (funding rate changes)
- Check emergency exit threshold
- Reduce position size or leverage

## Advanced Features (TODO)

- [ ] Multiple trading pairs
- [ ] Dynamic BPS threshold based on volatility
- [ ] Funding rate integration in strategy
- [ ] Telegram notifications
- [ ] Grafana dashboard
- [ ] Backtesting module

## License

MIT License - Use at your own risk

## Disclaimer

This bot is for educational purposes. Cryptocurrency trading involves significant risk. Always test thoroughly before deploying real capital. The authors are not responsible for any financial losses.

---

**Built with ❤️ in Rust 🦀**
