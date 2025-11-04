# Exit Mechanism - Detailed Flow

## 🎯 Overview

The bot uses a **2-stage exit strategy** to maximize fee efficiency while ensuring guaranteed position closure:

1. **Stage 1 (0-30s)**: ALO orders for maker rebate (-0.025% fee → +0.25% rebate)
2. **Stage 2 (30s+)**: IOC force close with 0.1% slippage buffer (guaranteed execution)

---

## 📊 Complete Trade Lifecycle Example

### **Entry Phase: ~50-100ms**

```
T+0ms     | BPS Monitoring (every 100ms)
          | Current: Perp Ask=$10.02, Spot Bid=$10.00
          | BPS = ((10.02-10.00)/10.00)*10000 = 20 BPS ✅
          |
T+1ms     | ✅ SIGNAL: BPS >= Threshold (20 >= 20)
          | ✅ Liquidity check passed
          | ✅ Risk limits ok
          |
T+2ms     | Building batch orders:
          | [1] SELL 30 HYPE @ $10.02 (PERP, 2x leverage, IOC)
          | [2] BUY  15 HYPE @ $10.00 (SPOT, 1x, IOC)
          |
T+5ms     | Signing batch transaction (same nonce → atomic)
          |
T+25ms    | API Response: BOTH FILLED ✅
          | Entry price logged: Perp=$10.02, Spot=$10.00
          | State: IDLE → POSITION_OPEN
          |
TOTAL: ~25-50ms (Mumbai to Hyperliquid)
```

**Why IOC for Entry?**
- Immediate execution guarantee
- No partial fills stuck on book
- Opportunity capture before spread changes
- Atomic execution (both orders or neither)

---

### **Position Monitoring Phase: 5-30 minutes**

```
Every 100ms:
    Current BPS = calculate_bps()

    Example Timeline:
    00:00 | BPS: 20.0  (entry)
    00:30 | BPS: 18.5
    01:00 | BPS: 15.2
    02:00 | BPS: 12.8
    05:00 | BPS: 8.3
    10:00 | BPS: 5.1
    15:00 | BPS: 2.4
    18:00 | BPS: 0.8
    19:45 | BPS: -0.2  ✅ EXIT SIGNAL (BPS <= 0)
```

**Profit Tracking:**
- Entry BPS: 20.0
- Exit BPS: -0.2
- **Gross Profit: 20.2 BPS**
- Position: 15 HYPE @ $10/coin
- **USD Profit: 20.2 * 0.0001 * 15 * 10 = $3.03**

---

### **Exit Phase: Stage 1 (ALO) - 0 to 30 seconds**

```
T+0s      | BPS = -0.2 detected ✅ EXIT SIGNAL!
          | Current: Perp Bid=$10.003, Spot Ask=$10.005
          |
T+0.01s   | Building ALO exit orders (maker only):
          | [1] BUY  30 HYPE @ $10.003 (PERP, ALO, reduce_only)
          | [2] SELL 15 HYPE @ $10.005 (SPOT, ALO, reduce_only)
          |
T+0.05s   | API Response: Orders placed on book ✅
          | State: POSITION_OPEN → WAITING_EXIT
          | exit_order_time = now()
          |
          | ⏰ 30-second countdown started
          |
--- WAITING FOR FILLS ---
          |
T+2.3s    | Perp order FILLED ✅ (someone hit our bid)
          | Spot order still pending...
          |
T+5.8s    | Spot order FILLED ✅ (someone hit our ask)
          |
T+5.85s   | ✅ BOTH FILLED! Calculating PnL...
          |
          | Entry: 20.0 BPS
          | Exit:  -0.2 BPS
          | Profit: 20.2 BPS = $3.03
          |
          | Fees:
          | Entry: 2 * 0.025% * $150 = -$0.075 (taker)
          | Exit:  2 * 0.25%  * $150 = +$0.75  (maker rebate!)
          | Net fees: +$0.675 ✅
          |
          | **TOTAL PROFIT: $3.03 + $0.68 = $3.71** ✅
          |
          | State: WAITING_EXIT → IDLE
          | Position cleared
          |
TOTAL EXIT: 5.85 seconds
```

**Why ALO for Exit?**
- **Maker rebate**: +0.25% instead of paying -0.025%
- **Better net profit**: +$0.75 fee income vs -$0.075 fee cost
- **No rush**: Spread stays near 0, plenty of time
- **Liquidity provision**: Bot becomes market maker

---

### **Exit Phase: Stage 2 (IOC Fallback) - 30+ seconds**

**Scenario: ALO orders didn't fill (low liquidity moment)**

```
T+30.0s   | ⚠️ TIMEOUT! ALO orders still pending
          | Perp: 50% filled, Spot: 0% filled
          |
T+30.01s  | 🚨 Cancelling pending ALO orders
          | cancel_all_orders(PERP) ✅
          | cancel_all_orders(SPOT) ✅
          |
T+30.05s  | Building IOC force close orders:
          | Current: Perp Bid=$10.002, Spot Ask=$10.004
          |
          | Adding 0.1% slippage buffer:
          | Perp price: $10.002 * 1.001 = $10.012 (buy higher)
          | Spot price: $10.004 * 0.999 = $9.994  (sell lower)
          |
          | [1] BUY  15 HYPE @ $10.012 (PERP, IOC, reduce_only)
          | [2] SELL 15 HYPE @ $9.994  (SPOT, IOC, reduce_only)
          |     (perp was 50% filled, closing remaining 15)
          |
T+30.10s  | API Request sent
          |
T+30.15s  | API Response: BOTH FILLED ✅ (IOC guarantee)
          |
T+30.16s  | ✅ FORCE CLOSE SUCCESSFUL!
          |
          | PnL Calculation:
          | Entry: 20.0 BPS
          | Exit:  1.8 BPS (worse due to slippage)
          | Profit: 18.2 BPS = $2.73
          |
          | Fees:
          | Entry: -$0.075 (taker)
          | Exit:  -$0.075 (taker, IOC)
          | Net fees: -$0.15
          |
          | **TOTAL PROFIT: $2.73 - $0.15 = $2.58** ✅
          | (Less than ALO exit but still profitable)
          |
          | State: WAITING_EXIT → IDLE
          |
TOTAL EXIT: 30.16 seconds
```

**Why IOC with Slippage?**
- **Guaranteed execution**: Cannot stay stuck in position
- **0.1% buffer**: Ensures fill even if book moves
- **Risk management**: Better small loss than big loss from stuck position
- **Fallback safety**: Manual intervention not needed

---

## ⚡ Performance Metrics

### **Entry**
| Metric | Value | Notes |
|--------|-------|-------|
| Signal Detection | 1-5ms | BPS calculation + checks |
| Order Creation | 1-2ms | Batch order building |
| API Round-trip | 20-50ms | Mumbai → Hyperliquid |
| **TOTAL** | **~50-100ms** | From signal to filled |

### **Exit (ALO - Normal)**
| Metric | Value | Notes |
|--------|-------|-------|
| Signal Detection | 1-5ms | BPS <= 0 check |
| Order Placement | 50ms | ALO batch order |
| Fill Wait | 0.5-20s | Market dependent |
| **TOTAL** | **0.5-20s** | Usually 2-10s |
| **Success Rate** | ~80% | In liquid conditions |

### **Exit (IOC - Fallback)**
| Metric | Value | Notes |
|--------|-------|-------|
| Timeout | 30s | ALO wait period |
| Cancel + IOC | 150-200ms | Cancel + new order |
| **TOTAL** | **~30.2s** | Guaranteed success |
| **Success Rate** | ~100% | Force close |

---

## 🛡️ Safety Mechanisms

### **1. Emergency Exit**
```rust
if spread_change > MAX_LOSS_BPS {
    // Spread widened too much → losing money
    // Cancel all orders immediately
    // Force close with IOC
}
```

**Trigger:** BPS increases by >50 BPS after entry
**Example:** Entry at 20 BPS, current at 75 BPS → losing money!

---

### **2. Critical Failure Handling**
```rust
if force_close_failed {
    error!("💀 CRITICAL: Manual intervention required!");
    // Keep position_size in memory
    // Retry on next loop (10ms later)
    // Log to alert system
}
```

**Fallback:** Even IOC can fail (network issue, insufficient margin)
**Action:** Bot retries automatically, logs critical error

---

### **3. Partial Fill Protection**
```rust
// Entry: Atomic batch (same nonce)
if perp_filled && !spot_filled {
    // This shouldn't happen with atomic batch
    // But if it does, cancel perp immediately
}
```

---

## 💰 Fee Optimization Strategy

### **Best Case (ALO Exit)**
```
Entry:  -0.025% taker  * $150 * 2 = -$0.075
Exit:   +0.25%  maker  * $150 * 2 = +$0.75
Net:    +$0.675 ✅

Minimum profitable BPS: ~5 BPS
(to cover any slippage)
```

### **Worst Case (IOC Exit)**
```
Entry:  -0.025% taker  * $150 * 2 = -$0.075
Exit:   -0.025% taker  * $150 * 2 = -$0.075
        +0.1%   slippage          = -$0.30
Net:    -$0.45 ❌

Minimum profitable BPS: ~30 BPS
(to cover fees + slippage)
```

### **Strategy Balance**
- **Target 20 BPS threshold**: Good buffer over worst case
- **ALO first**: Try to get maker rebate (80% success)
- **IOC fallback**: Guarantee exit (20% of trades)
- **Average fee**: ~+$0.40 per trade

---

## 📈 Expected Trade Statistics

Assuming 10 trades/day with $150 position:

| Metric | Value | Notes |
|--------|-------|-------|
| Avg Entry BPS | 22 BPS | ~$3.30/trade |
| Avg Exit BPS | -0.5 BPS | Profit: 22.5 BPS |
| Avg Gross P&L | $3.38/trade | 22.5 * $150 |
| ALO Exit Rate | 80% | 8/10 trades |
| Avg Fees (net) | +$0.35/trade | Mostly maker |
| **Net P&L/trade** | **$3.73** | After fees |
| **Daily P&L** | **$37.30** | 10 trades |
| **Win Rate** | 85-90% | Some trades hit stop |
| **Monthly P&L** | **~$1,120** | $150 capital |
| **ROI** | **746%/year** | Theoretical max |

⚠️ **Note**: Real results depend on:
- Spread availability (how often BPS > 20)
- Funding rates (can be +/-)
- Slippage and partial fills
- Market volatility

---

## 🔧 Configuration Tuning

### **Aggressive (More Trades, More Risk)**
```env
BPS_THRESHOLD=15.0        # Lower threshold
TIMEOUT_SECONDS=20        # Faster IOC fallback
MAX_LOSS_BPS=35.0         # Tighter stop
```
- More entry opportunities
- Higher rejected order rate
- Lower avg profit/trade

### **Conservative (Fewer Trades, Higher Quality)**
```env
BPS_THRESHOLD=25.0        # Higher threshold
TIMEOUT_SECONDS=45        # More time for ALO
MAX_LOSS_BPS=60.0         # Wider stop
```
- Fewer entries (higher quality)
- Better ALO fill rate
- Higher avg profit/trade

### **Recommended (Balanced)**
```env
BPS_THRESHOLD=20.0        # Sweet spot
TIMEOUT_SECONDS=30        # Good balance
MAX_LOSS_BPS=50.0         # Safe stop
```

---

## 🐛 Debugging Exit Issues

### **Issue: ALO never fills**
```bash
# Check logs for:
"Exit signal! BPS: X"
"Placing 2 exit orders (ALO)"
"⏰ ALO timeout, forcing close with IOC"  # Should see this at 30s
```
**Cause**: Low liquidity, spread moving away
**Fix**: Reduce TIMEOUT_SECONDS to 15-20s

---

### **Issue: IOC fails**
```bash
# Check logs for:
"❌ Force close FAILED: ..."
"💀 CRITICAL: Manual intervention required!"
```
**Cause**: Insufficient margin, API error, network issue
**Action**: Bot auto-retries, but check position manually

---

### **Issue: Stuck in WAITING_EXIT**
```bash
# Check state:
Strategy state: POSITION_OPEN -> WAITING_EXIT
# Should transition to IDLE within 30s
```
**Debug**: Check if exit_order_time is being set correctly

---

## 📝 Code Reference

- **Entry Logic**: `src/main.rs:154-174`
- **Exit Signal**: `src/strategy.rs:86-109`
- **ALO Exit**: `src/main.rs:177-201`
- **IOC Fallback**: `src/main.rs:230-272`
- **Build Exit Orders**: `src/strategy.rs:204-284`

---

## ✅ Exit Checklist

- [x] ALO orders for fee efficiency
- [x] 30-second timeout before fallback
- [x] IOC with 0.1% slippage buffer
- [x] Emergency exit on spread widening
- [x] Critical failure retry logic
- [x] PnL tracking for both exit types
- [x] Order cancellation before force close
- [x] Atomic batch execution
- [x] Comprehensive logging

---

**Built with ❤️ for maximum exit flexibility and safety** 🦀
