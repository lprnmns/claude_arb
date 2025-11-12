#!/bin/bash

# Hyperliquid Arbitrage Bot - Quick Deployment Script
# This script stops the bot, pulls latest code, rebuilds, and restarts

set -e  # Exit on error

echo "🚀 Hyperliquid Arbitrage Bot - Deployment Starting..."
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
BRANCH="claude/hyperliquid-arbitrage-bot-011CUoTEXudVdnxzHRTb2fUV"
TARGET_COMMIT="175b991"  # Latest commit with all fixes
BOT_DIR="/home/ubuntu/claude_arb"
BOT_NAME="hyperliquid-arb-bot"

# Step 1: Stop running bot
echo -e "${YELLOW}📌 Step 1/6: Stopping running bot...${NC}"
pkill $BOT_NAME || echo "   (No bot was running)"
sleep 2
echo -e "${GREEN}✅ Bot stopped${NC}"
echo ""

# Step 2: Navigate to bot directory
echo -e "${YELLOW}📌 Step 2/6: Navigating to bot directory...${NC}"
cd $BOT_DIR
echo -e "${GREEN}✅ In directory: $(pwd)${NC}"
echo ""

# Step 3: Show current status
echo -e "${YELLOW}📌 Step 3/6: Checking current status...${NC}"
CURRENT_COMMIT=$(git log -1 --oneline | awk '{print $1}')
echo "   Current commit: $CURRENT_COMMIT"
git status --short
echo ""

# Step 4: Pull latest code
echo -e "${YELLOW}📌 Step 4/6: Pulling latest code...${NC}"
git fetch origin $BRANCH
git reset --hard $TARGET_COMMIT

NEW_COMMIT=$(git log -1 --oneline | awk '{print $1}')
echo -e "${GREEN}✅ Now on commit: $NEW_COMMIT${NC}"
git log -1 --oneline
echo ""

# Step 5: Rebuild
echo -e "${YELLOW}📌 Step 5/6: Building release binary...${NC}"
cargo build --release
echo -e "${GREEN}✅ Build complete${NC}"
echo ""

# Step 6: Start bot
echo -e "${YELLOW}📌 Step 6/6: Starting bot...${NC}"
RUST_LOG=info ./target/release/$BOT_NAME > bot.log 2>&1 &
BOT_PID=$!
sleep 3

# Check if bot is still running
if ps -p $BOT_PID > /dev/null; then
    echo -e "${GREEN}✅ Bot started successfully (PID: $BOT_PID)${NC}"
else
    echo -e "${RED}❌ Bot failed to start! Check bot.log for errors${NC}"
    exit 1
fi
echo ""

# Show initial logs
echo -e "${YELLOW}📊 Initial logs (first 30 lines):${NC}"
echo "=================================================="
head -30 bot.log
echo "=================================================="
echo ""

echo -e "${GREEN}🎉 Deployment complete!${NC}"
echo ""
echo "📝 Useful commands:"
echo "   tail -f bot.log          # Follow live logs"
echo "   tail -100 bot.log | grep BPS  # Check BPS values"
echo "   ps aux | grep $BOT_NAME  # Check if bot is running"
echo "   pkill $BOT_NAME          # Stop the bot"
echo ""
echo "✅ Next: Monitor logs with: tail -f bot.log"
