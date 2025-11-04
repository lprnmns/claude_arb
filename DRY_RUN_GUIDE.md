# Dry-Run Mode (Paper Trading) Guide

## 🎯 Ne İşe Yarar?

**DRY RUN mode**, gerçek para riski olmadan botun tüm sistemini test etmenizi sağlar:

✅ **Gerçek market verisi**: WebSocket'ten live orderbook
✅ **Gerçek BPS hesaplamaları**: Actual spread'ler
✅ **Gerçek timing**: Entry/exit logic
✅ **Simüle edilmiş emirler**: API'ye gönderilmez
✅ **Simüle edilmiş P&L**: Kar/zarar tracking
✅ **Tam statistics**: Win rate, trades, daily P&L

❌ **API'ye order göndermez**
❌ **Bakiye harcamaz**
❌ **Risk yok**

---

## 🚀 Hızlı Başlangıç

### 1. DRY RUN Aktifleştir

`.env` dosyasında:
```env
DRY_RUN=true
DRY_RUN_FILL_DELAY_MS=50        # Simüle edilmiş latency
DRY_RUN_FILL_SUCCESS_RATE=95    # %95 orders fill olur
```

### 2. Botu Çalıştır

```bash
cargo run

# Veya release mode (daha hızlı)
cargo build --release
./target/release/hyperliquid-arb-bot
```

### 3. Beklenen Çıktı

```
🚀 Hyperliquid Arbitrage Bot starting...
✅ Configuration loaded successfully
   Wallet: 0x7e5f...95bdf
   BPS Threshold: 20.0
   Position Size: $150.0
   Leverage: 2x
🔶 DRY RUN MODE ENABLED 🔶
   Orders will be SIMULATED (no real trades)
   Fill delay: 50ms
   Fill success rate: 95%
   Set DRY_RUN=false for real trading
🔧 Initializing components...
✅ API client initialized
📡 Subscribing to orderbook updates...
✅ WebSocket connected and subscribed to HYPE
⏳ Waiting for initial orderbook data...
✅ All components initialized
🎯 Starting trading loop...
   Press Ctrl+C to stop
```

---

## 📊 Test Senaryoları

### **Senaryo 1: BPS Monitoring (İlk 5 Dakika)**

Bot çalıştıkça her 100ms'de BPS hesaplıyor. Logları izleyin:

```bash
# Terminal 1: Bot çalıştır
cargo run

# Terminal 2: Logları filtrele
tail -f hyperliquid-arb-bot.log | grep "Current BPS"
```

**Beklenen:**
```
📊 Current BPS: 15.32
📊 Current BPS: 14.89
📊 Current BPS: 16.21
📊 Current BPS: 19.45
📊 Current BPS: 22.31  ← 20 BPS threshold geçildi!
```

**Değerlendirme:**
- BPS değerleri gerçek mi? ✅
- 20 BPS'e kaç dakikada bir ulaşıyor? 📈
- Spread pattern nasıl? (sürekli dar mı, ani spike'lar mı?)

---

### **Senaryo 2: Entry Simulation**

20 BPS threshold geçildiğinde entry simüle edilecek:

```
🎯 Entry signal! BPS: 22.5 (threshold: 20.0)
🚀 Executing entry...
Placing 2 entry orders (IOC)
📝 DRY RUN: Simulating entry orders...
📝 DRY RUN: Order 1 FILLED - SELL 30.5 @ $10.023
📝 DRY RUN: Order 2 FILLED - BUY 15.25 @ $10.001
✅ DRY RUN: Entry execution simulated successfully
✅ Entry successful! Position size: 30.5
Strategy state: Idle -> PositionOpen
Entry BPS recorded: 22.5
```

**Timing Testi:**
- Entry detection → execution: **<100ms** ✅
- Order fill simulation: **50ms** (config'den)

**Değerlendirme:**
- Entry BPS değeri mantıklı mı?
- Position sizing doğru mu? (150 USDC / price * leverage)
- Fill simulation gerçekçi mi?

---

### **Senaryo 3: Position Hold & Exit Simulation**

Entry'den sonra BPS 0'a düşene kadar bekleyecek:

```
[Position Hold Phase]
Current BPS: 22.5  (entry)
Current BPS: 20.1
Current BPS: 18.3
Current BPS: 15.7
Current BPS: 12.4
Current BPS: 8.9
Current BPS: 5.2
Current BPS: 2.1
Current BPS: 0.3
Current BPS: -0.4  ← Exit trigger!

🎯 Exit signal! BPS: -0.4 (target: 0)
🎯 Executing exit...
Placing 2 exit orders (ALO - maker only)
📝 DRY RUN: Simulating ALO exit orders...
📝 DRY RUN: ALO Order 1 PLACED - BUY 30.5 @ $10.004
📝 DRY RUN: ALO Order 2 PLACED - SELL 15.25 @ $10.006
📝 DRY RUN: Waiting for ALO fills (~3500ms)...
✅ DRY RUN: ALO orders filled (simulated)
✅ Exit successful!

[PnL Calculation]
Entry BPS: 22.5
Current BPS: -0.4
Profit: 22.9 BPS = $3.43
📊 Risk Statistics:
   Daily PnL: $3.43
   Total Trades: 1
   Win Rate: 100.0%
   Avg Win: $3.43

Strategy state: PositionOpen -> Idle
```

**Değerlendirme:**
- Hold time ne kadar? (typically 5-30 dakika)
- ALO fill simulation: 2-10 saniye arasında
- P&L hesaplama doğru mu?

---

### **Senaryo 4: ALO Timeout → IOC Fallback**

Bazı durumlarda ALO 30 saniyede fill olmaz, IOC fallback tetiklenir:

```
🎯 Executing exit...
📝 DRY RUN: Simulating ALO exit orders...
📝 DRY RUN: Waiting for ALO fills (~9000ms)...

[30 seconds pass without fill]

⏰ ALO timeout, forcing close with IOC
🚨 Force closing with IOC (0.1% slippage buffer)
Placing 2 force close orders (IOC with 0.1% slippage)
📝 DRY RUN: Simulating IOC force close...
📝 DRY RUN: IOC Order 1 FILLED - BUY 30.5 @ $10.014 (with slippage)
📝 DRY RUN: IOC Order 2 FILLED - SELL 15.25 @ $9.996 (with slippage)
✅ DRY RUN: Force close successful (simulated)
✅ Force close successful!

Profit with slippage: $3.10 (vs $3.43 with ALO)
⚠️ Exit with IOC slippage: ~$3.10
```

**Not:** IOC fallback'i test etmek için `.env`'de:
```env
TIMEOUT_SECONDS=5  # 30 yerine 5 saniye yapın
```

---

### **Senaryo 5: Emergency Exit**

Spread çok genişlerse emergency exit tetiklenir:

```
[Position Hold]
Entry BPS: 20.0
Current BPS: 25.3
Current BPS: 35.8
Current BPS: 52.1  ← Spread widened!
Current BPS: 71.2  ← MAX_LOSS_BPS (50) exceeded!

⚠️ Emergency exit triggered! BPS change: 51.2
⚠️ EMERGENCY EXIT!
[Force close with IOC immediately]

Loss recorded: -$2.50
```

**Test Etmek İçin:**
- MAX_LOSS_BPS'i düşür: `MAX_LOSS_BPS=20.0`
- Volatil bir zamanda çalıştır

---

### **Senaryo 6: Order Rejection Simulation**

%5 ihtimalle order reject simüle edilir:

```
🎯 Entry signal! BPS: 21.3
🚀 Executing entry...
📝 DRY RUN: Simulating entry orders...
📝 DRY RUN: Simulated REJECTION (95% fill rate)
❌ Entry failed: Simulated order rejection
Strategy state: Idle
```

**Değerlendirme:**
- Rejection handling doğru mu?
- Bot crash olmadan devam ediyor mu?
- Retry yok, bir sonraki opportunity'de tekrar dener

---

## 📈 Performans Metrikleri

### **1 Saatlik Test Sonuçları (Beklenen)**

```
Runtime: 1 hour
BPS > 20: 8-12 times
Entry attempts: 8-12
Successful entries: 7-11 (95% fill rate)
Avg hold time: 15 minutes
Total trades: 7-11
Win rate: 85-90%
Avg profit/trade: $3.20
Total daily P&L (projected): $22-35
```

### **24 Saatlik Test (Önerilen)**

```bash
# Background'da çalıştır
screen -S arb-test
cargo run

# Detach: Ctrl+A, D

# 24 saat sonra kontrol et
screen -r arb-test
# Ctrl+C ile durdur

# Logları analiz et
grep "Risk Statistics" logs.txt | tail -5
```

**Beklenen Metrikler:**
- Total trades: 10-30
- Win rate: 80-90%
- Avg profit/trade: $2.50-4.00
- Daily P&L: $25-120 (simüle)

---

## 🔍 Ne İzlemeli?

### **Kritik Metrikler:**

1. **BPS Frequency**
   ```bash
   grep "Current BPS" logs.txt | awk '{print $NF}' | \
     awk '$1 > 20 {count++} END {print "Times > 20 BPS:", count}'
   ```
   - Hedef: Saatte 2+ kez

2. **Entry Success Rate**
   ```bash
   grep "Entry successful" logs.txt | wc -l
   grep "Entry failed" logs.txt | wc -l
   ```
   - Hedef: >90%

3. **Avg Hold Time**
   ```bash
   # Entry timestamp - Exit timestamp
   grep -E "Entry successful|Exit successful" logs.txt
   ```
   - Hedef: 5-30 dakika

4. **ALO Fill Rate**
   ```bash
   grep "ALO orders filled" logs.txt | wc -l
   grep "ALO timeout" logs.txt | wc -l
   ```
   - Hedef: >70% ALO, <30% IOC fallback

---

## ⚠️ Yaygın Sorunlar

### **1. WebSocket Disconnects**
```
❌ WebSocket error for HYPE: Connection closed
Reconnecting in 5s...
```
**Çözüm:** Normal, otomatik reconnect ediyor

---

### **2. No Entry Signals**
```
[30 minutes running, no entries]
📊 Current BPS: 15.2
📊 Current BPS: 14.8
```
**Neden:** Spread dar, threshold'a ulaşmıyor
**Çözüm:**
- Threshold'u düşür: `BPS_THRESHOLD=15.0`
- Veya volatil bir zaman dilimi bekle

---

### **3. Simulated P&L vs Reality**
Dry-run P&L **gerçek sonuçlardan farklı** olabilir:
- ❌ Slippage gerçekte daha fazla olabilir
- ❌ ALO fill rate gerçekte daha düşük olabilir
- ❌ Funding rate dahil değil
- ❌ Market impact hesaplanmıyor

**Dry-run sadece mantık testi içindir!**

---

## ✅ Live Trading'e Geçiş Kriterleri

Dry-run'dan sonra live'a geçmeden önce:

- [ ] 24 saat crash olmadan çalıştı
- [ ] BPS hesaplamaları mantıklı (manuel karşılaştırma)
- [ ] Entry/exit logic doğru (log review)
- [ ] Statistics doğru hesaplanıyor
- [ ] WebSocket stable (disconnect-reconnect çalışıyor)
- [ ] Emergency exit test edildi
- [ ] ALO timeout → IOC fallback test edildi
- [ ] Win rate >80% (simüle)
- [ ] Kod anlaşıldı ve güven var

**Sonra:**
```env
# .env
DRY_RUN=false  # ⚠️ Real trading!
POSITION_SIZE_USD=10.0  # İlk test için küçük
```

10-50 USDC ile 1-2 gün test → Sonra 100-300 USDC'ye çık.

---

## 📝 Log Dosyası Oluşturma

```bash
# Logları file'a kaydet
cargo run 2>&1 | tee hyperliquid-arb-bot.log

# Veya background'da
cargo run > bot.log 2>&1 &

# Logları izle
tail -f bot.log
```

---

## 🎯 Sonuç

**DRY RUN mode sayesinde:**
- ✅ Sistemin tamamını test edebilirsiniz
- ✅ BPS spread frequency'i görebilirsiniz
- ✅ Timing ve logic'i doğrulayabilirsiniz
- ✅ Risk almadan deneyim kazanırsınız
- ✅ Code'a güven oluşur

**Tavsiye:** En az 1-2 gün dry-run, sonra 10 USDC real test, sonra production.

---

**Happy Testing! 🦀📊**
