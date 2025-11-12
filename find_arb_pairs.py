#!/usr/bin/env python3
"""
Find coins that have BOTH perp and spot markets for arbitrage
"""
import requests
import json

API_URL = "https://api.hyperliquid.xyz/info"

def get_meta():
    payload = {"type": "meta"}
    return requests.post(API_URL, json=payload).json()

def get_spot_meta():
    payload = {"type": "spotMeta"}
    return requests.post(API_URL, json=payload).json()

def main():
    print("=" * 80)
    print("COINS WITH BOTH PERP AND SPOT MARKETS (FOR ARBITRAGE)")
    print("=" * 80)
    print()

    # Get perp markets
    meta = get_meta()
    perp_coins = set()
    if "universe" in meta:
        for market in meta["universe"]:
            perp_coins.add(market.get("name"))

    print(f"📊 Found {len(perp_coins)} PERP markets")
    print()

    # Get spot tokens
    spot_meta = get_spot_meta()
    spot_coins = set()
    spot_markets = {}

    if "tokens" in spot_meta:
        for token in spot_meta["tokens"]:
            if isinstance(token, dict):
                name = token.get("name")
                if name:
                    spot_coins.add(name)

    if "universe" in spot_meta:
        for market in spot_meta["universe"]:
            if isinstance(market, dict):
                name = market.get("name")
                if name and "/" in name:  # TOKEN/USDC format
                    token = name.split("/")[0]
                    spot_markets[token] = name

    print(f"📊 Found {len(spot_coins)} SPOT tokens")
    print(f"📊 Found {len(spot_markets)} SPOT markets (TOKEN/USDC)")
    print()

    # Find coins with BOTH
    arb_pairs = []
    for coin in perp_coins:
        if coin in spot_markets:
            arb_pairs.append({
                "coin": coin,
                "perp": coin,
                "spot": spot_markets[coin]
            })

    print("=" * 80)
    print(f"✅ COINS WITH BOTH PERP AND SPOT ({len(arb_pairs)} found):")
    print("=" * 80)
    print()

    if arb_pairs:
        for i, pair in enumerate(arb_pairs[:20], 1):  # Show first 20
            print(f"{i:2}. {pair['coin']:10} → PERP: {pair['perp']:15} SPOT: {pair['spot']}")

        print()
        print("💡 RECOMMENDED PAIRS FOR HIGH VOLUME:")
        print()

        # Highlight major coins
        major_coins = ["BTC", "ETH", "SOL", "PURR"]
        for coin in major_coins:
            for pair in arb_pairs:
                if pair["coin"] == coin:
                    print(f"   🎯 {pair['coin']:10} → PERP: '{pair['perp']}'  SPOT: '{pair['spot']}'")

        print()
        print("=" * 80)
        print()
        print("📝 TO USE IN YOUR BOT:")
        print()
        print("Update .env:")
        print("  PERP_SYMBOL=PURR    # Use coin name for perp")
        print("  SPOT_SYMBOL=PURR/USDC  # Use 'COIN/USDC' format for spot")
        print()
        print("Your bot will then subscribe to:")
        print("  - PERP: 'PURR' ")
        print("  - SPOT: 'PURR/USDC'")
        print()

    else:
        print("❌ NO COINS FOUND WITH BOTH PERP AND SPOT!")
        print()
        print("This might mean:")
        print("  1. Spot markets use different naming (not TOKEN/USDC)")
        print("  2. Very few coins have spot markets")
        print()
        print("First few spot markets:")
        for market in list(spot_markets.items())[:10]:
            print(f"   - {market[0]} → {market[1]}")

    print()
    print("=" * 80)

if __name__ == "__main__":
    main()
