# 🚀 Server Deployment Guide

## Current Status

**Latest Commit:** `ad86b52` 🚀 **ALL SYSTEMS GO - Bot ready for live trading!**
**Branch:** `claude/hyperliquid-arbitrage-bot-011CUoTEXudVdnxzHRTb2fUV`

## ✅ What's Fixed

1. **✅ Spot Market Symbol Mapping**
   - HYPE/USDC → @107 automatic mapping
   - No more "market not found" errors

2. **✅ Agent Mode**
   - `vaultAddress: null` (no vault registration needed)
   - Saves 100 USDC vault fee

3. **✅ Correct BPS Calculation**
   - Real perp-spot arbitrage (not internal spread)
   - HYPE PERP vs HYPE SPOT (@107)

4. **✅ Enhanced WebSocket Diagnostics**
   - Comprehensive message logging
   - Shows raw messages, parsed data, and coin field values
   - Helps diagnose @107 spot market data processing

5. **✅ RUST_LOG Environment Variable** (Commit `352c7d5`)
   - Fixed tracing_subscriber to respect RUST_LOG setting
   - Previous code hardcoded INFO level, ignoring RUST_LOG=debug
   - Now debug logs will actually appear when using RUST_LOG=debug

6. **✅ CRITICAL: Orderbook Snapshot Mode** (Commit `ad86b52`)
   - Fixed BPS using stale prices (e.g., -85 BPS bug)
   - Hyperliquid sends SNAPSHOTS, not deltas
   - Old code was accumulating price levels instead of replacing them
   - Example bug: WebSocket shows bid $39.937, BPS used cached bid $40.233
   - Now: Orderbook clears and replaces all levels on each update
   - **This fixes the "BPS stuck at constant value" bug!**

7. **✅ FINAL FIX: Spot Order Symbol Mapping** (Commit `ad86b52`) **READY TO TRADE!**
   - Fixed "Unknown symbol: HYPE/USDC" API errors
   - Order API now uses mapped symbol (@107) instead of config symbol (HYPE/USDC)
   - All order functions updated: entry, exit, force close, cancel
   - Bot can now execute real trades! 🎯

---

## 📋 Step-by-Step Deployment

### 1️⃣ **SSH to Server**

```bash
ssh -i ./ssh.key ubuntu@161.118.168.33
```

### 2️⃣ **Stop Running Bot**

```bash
pkill hyperliquid-arb-bot
```

### 3️⃣ **Check Current State**

```bash
cd /home/ubuntu/claude_arb

# What commit are you on?
git log -1 --oneline

# Any local changes?
git status
```

**Expected:** You should be on an older commit (like `07c2d21`) which has the vault bug.

### 4️⃣ **Pull Latest Code**

```bash
# Fetch latest
git fetch origin claude/hyperliquid-arbitrage-bot-011CUoTEXudVdnxzHRTb2fUV

# Reset to latest (WARNING: Discards local changes!)
git reset --hard ad86b52

# Verify
git log -1 --oneline
# Should show: "ad86b52 fix: Use mapped spot symbol (@107) for order API calls"
```

### 5️⃣ **Update .env File**

```bash
nano .env
```

**Required changes:**

```bash
# Agent wallet (REQUIRED)
HL_API_AGENT_PRIVATE_KEY=your_actual_agent_private_key

# Symbols - HYPE/USDC works now!
PERP_SYMBOL=HYPE
SPOT_SYMBOL=HYPE/USDC  # Will auto-map to @107

# Threshold
BPS_THRESHOLD=5.0  # Start with 5 BPS for testing

# Position size
POSITION_SIZE_USD=20.0

# Live trading
DRY_RUN=false
MANUAL_TEST_TRADE=false
```

**Save:** Press `Ctrl+X`, then `Y`, then `Enter`

### 6️⃣ **Rebuild Binary**

```bash
cargo build --release
```

**Expected:** Should take 30-60 seconds. Watch for "Finished" message.

### 7️⃣ **Start Bot**

```bash
RUST_LOG=info ./target/release/hyperliquid-arb-bot > bot.log 2>&1 &
```

### 8️⃣ **Monitor Logs**

```bash
tail -f bot.log
```

---

## 🔍 **What to Look For in Logs**

### ✅ **Good Signs:**

```log
✅ Configuration loaded successfully
   Agent Wallet: 0x05acb7e4ed6c51929059f44c8f710ea7558bf861

📍 Spot symbol mapping: HYPE/USDC → @107

📡 Subscribing to Hyperliquid L2Book - Orderbook: HYPE-PERP, Coin Symbol: HYPE
📡 Subscribing to Hyperliquid L2Book - Orderbook: HYPE-SPOT, Coin Symbol: @107

✅ WebSocket connected - Orderbook: HYPE-PERP, Symbol: HYPE
✅ WebSocket connected - Orderbook: HYPE-SPOT, Symbol: @107

📨 Raw WebSocket message for HYPE-PERP (first 300 chars): ...
📬 Parsed message for HYPE-PERP - Channel: l2Book, has data: true
📊 L2Book data for HYPE-PERP - Coin field: HYPE, Levels count: 2
🔄 Starting orderbook update - HYPE-PERP | Coin: HYPE | Bids: 20, Asks: 20
  📈 Best BID: $24.5678 x 1000
  📉 Best ASK: $24.5680 x 500
✅ Orderbook updated - HYPE-PERP | Coin: HYPE | Bids: 20, Asks: 20

📨 Raw WebSocket message for HYPE/USDC-SPOT (first 300 chars): ...
📬 Parsed message for HYPE/USDC-SPOT - Channel: l2Book, has data: true
📊 L2Book data for HYPE/USDC-SPOT - Coin field: @107, Levels count: 2
🔄 Starting orderbook update - HYPE/USDC-SPOT | Coin: @107 | Bids: 20, Asks: 20
  📈 Best BID: $24.5650 x 800
  📉 Best ASK: $24.5652 x 600
✅ Orderbook updated - HYPE/USDC-SPOT | Coin: @107 | Bids: 20, Asks: 20

✅ All components initialized
🎯 Starting trading loop...

📊 Current BPS: 2.48  ← Positive BPS!
```

### ❌ **Bad Signs (Old Code):**

```log
❌ "vaultAddress": "0xdC684e809b8beD7796d56c0877dA610aF108CAb7"
   ^ Should be "vaultAddress": null

❌ "Vault not registered"
   ^ Old code with vault bug

❌ Coin Symbol: HYPE (for spot)
   ^ Should be: Coin Symbol: @107

❌ Current BPS: -100.00
   ^ Should be positive or small negative (not -100!)
```

---

## 🐛 **Troubleshooting**

### Problem 1: "Vault not registered"

**Cause:** Running old code with vault address
**Solution:**
```bash
git log -1 --oneline
# If NOT "ad86b52", you need to pull latest!
git reset --hard ad86b52
cargo build --release
```

### Problem 2: BPS always negative (-100)

**Cause:** Spot market not subscribing correctly
**Check log for:**
```log
📍 Spot symbol mapping: HYPE/USDC → @107  ← Should see this!
✅ WebSocket connected - Orderbook: HYPE-SPOT, Symbol: @107  ← Should be @107!
```

**If missing:** Pull latest code

### Problem 3: No orderbook updates (WebSocket connected but no data)

**Symptom:** Bot shows `✅ WebSocket connected` but NO `📨 Raw WebSocket message` logs
**Cause:** WebSocket isn't receiving messages OR messages are being filtered out

**Check with enhanced debug logging:**
```bash
# Look for raw WebSocket messages
tail -100 bot.log | grep "📨 Raw WebSocket"

# If NONE appear, messages aren't arriving
# If they appear for PERP but not SPOT, spot subscription failed
```

**What debug logs reveal:**
- `📨 Raw WebSocket message` - Message arrived
- `📬 Parsed message` - Message parsed successfully
- `📊 L2Book data` - Shows coin field value
- `🔄 Starting orderbook update` - Update begins
- `✅ Orderbook updated` - Update completed

**If no messages for @107:**
Check subscription response in logs:
```bash
grep "Subscription confirmed" bot.log
```

### Problem 4: No orderbook data

**Check:**
```bash
# Is bot running?
ps aux | grep hyperliquid-arb-bot

# Check log for errors
tail -100 bot.log | grep ERROR
```

---

## 📊 **Expected Performance**

### BPS Range:
- **Typical:** -10 to +10 BPS
- **Opportunity:** > 5 BPS
- **Rare:** > 20 BPS

### Current Live BPS:
```bash
# Check from your local machine:
python3 - <<'EOF'
import requests
API_URL = "https://api.hyperliquid.xyz/info"
perp = requests.post(API_URL, json={"type": "l2Book", "coin": "HYPE"}).json()
spot = requests.post(API_URL, json={"type": "l2Book", "coin": "@107"}).json()
perp_ask = float(perp["levels"][1][0]["px"])
spot_bid = float(spot["levels"][0][0]["px"])
bps = ((perp_ask - spot_bid) / spot_bid) * 10000
print(f"Live BPS: {bps:.2f}")
EOF
```

---

## 🎯 **Success Checklist**

- [ ] On commit `ad86b52` or later (CRITICAL for BPS accuracy!)
- [ ] `.env` has `HL_API_AGENT_PRIVATE_KEY` filled
- [ ] `.env` has `SPOT_SYMBOL=HYPE/USDC`
- [ ] Bot logs show `vaultAddress: null`
- [ ] Bot logs show `Coin Symbol: @107` for spot
- [ ] Bot logs show `📨 Raw WebSocket message` for both PERP and SPOT
- [ ] Bot logs show `✅ Orderbook updated` for both orderbooks
- [ ] BPS values are realistic (-10 to +10 range)
- [ ] No "Vault not registered" errors
- [ ] Orders being placed (when BPS > threshold)

---

## 📞 **Getting Help**

If bot still not working after following this guide:

1. **Capture logs:**
   ```bash
   head -200 bot.log > debug.log
   cat debug.log
   ```

2. **Check commit:**
   ```bash
   git log -1 --oneline
   ```

3. **Share both** with developer

---

**Last Updated:** 2025-11-12
**Commit:** ad86b52
