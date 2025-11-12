#!/bin/bash

# Hyperliquid Arbitrage Bot - Health Check Script
# Quickly verify bot status and health

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

BOT_NAME="hyperliquid-arb-bot"

echo "🔍 Hyperliquid Arbitrage Bot - Health Check"
echo "=============================================="
echo ""

# 1. Check if bot is running
echo -e "${YELLOW}1. Process Status:${NC}"
if pgrep -x $BOT_NAME > /dev/null; then
    PID=$(pgrep -x $BOT_NAME)
    echo -e "   ${GREEN}✅ Bot is running (PID: $PID)${NC}"
else
    echo -e "   ${RED}❌ Bot is NOT running${NC}"
fi
echo ""

# 2. Check current commit
echo -e "${YELLOW}2. Current Commit:${NC}"
COMMIT=$(git log -1 --oneline)
echo "   $COMMIT"
echo ""

# 3. Check for critical errors in logs
echo -e "${YELLOW}3. Recent Errors (last 50 lines):${NC}"
ERROR_COUNT=$(tail -50 bot.log | grep -c "ERROR\|❌" || true)
if [ $ERROR_COUNT -eq 0 ]; then
    echo -e "   ${GREEN}✅ No errors found${NC}"
else
    echo -e "   ${RED}❌ Found $ERROR_COUNT errors:${NC}"
    tail -50 bot.log | grep "ERROR\|❌" | tail -5
fi
echo ""

# 4. Check asset info loaded
echo -e "${YELLOW}4. Asset Info Status:${NC}"
if grep -q "Asset info loaded - Perp index: 159, Spot index: 107" bot.log; then
    echo -e "   ${GREEN}✅ Asset info loaded correctly${NC}"
    grep "Asset info loaded" bot.log | tail -1
else
    echo -e "   ${RED}❌ Asset info not found in logs${NC}"
fi
echo ""

# 5. Check orderbook updates
echo -e "${YELLOW}5. Orderbook Updates (last 5):${NC}"
tail -100 bot.log | grep "Orderbook updated" | tail -5 || echo "   No orderbook updates found"
echo ""

# 6. Check recent BPS values
echo -e "${YELLOW}6. Recent BPS Values (last 10):${NC}"
tail -100 bot.log | grep "Current BPS:" | tail -10 || echo "   No BPS values found yet"
echo ""

# 7. Check for order attempts
echo -e "${YELLOW}7. Order Activity (last 5):${NC}"
ORDER_COUNT=$(grep -c "Placing.*order\|Entry signal\|Exit signal" bot.log || true)
echo "   Total order-related events: $ORDER_COUNT"
if [ $ORDER_COUNT -gt 0 ]; then
    tail -100 bot.log | grep "Placing.*order\|Entry signal\|Exit signal" | tail -5
fi
echo ""

# 8. Summary
echo "=============================================="
echo -e "${YELLOW}Summary:${NC}"
if pgrep -x $BOT_NAME > /dev/null && [ $ERROR_COUNT -eq 0 ]; then
    echo -e "${GREEN}✅ Bot is healthy and running!${NC}"
elif pgrep -x $BOT_NAME > /dev/null; then
    echo -e "${YELLOW}⚠️  Bot is running but has some errors${NC}"
else
    echo -e "${RED}❌ Bot is not running${NC}"
fi
echo ""
echo "For live monitoring: tail -f bot.log"
