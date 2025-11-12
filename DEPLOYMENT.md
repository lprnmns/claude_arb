# 🚀 Server Deployment Guide

## Current Status

**Latest Commit:** `38f0d7c`
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
git reset --hard 38f0d7c

# Verify
git log -1 --oneline
# Should show: "38f0d7c feat: Add spot market symbol mapping"
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
# If NOT "38f0d7c", you need to pull latest!
git reset --hard 38f0d7c
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

### Problem 3: No orderbook data

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

- [ ] On commit `38f0d7c` or later
- [ ] `.env` has `HL_API_AGENT_PRIVATE_KEY` filled
- [ ] `.env` has `SPOT_SYMBOL=HYPE/USDC`
- [ ] Bot logs show `vaultAddress: null`
- [ ] Bot logs show `Coin Symbol: @107` for spot
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
**Commit:** 38f0d7c
