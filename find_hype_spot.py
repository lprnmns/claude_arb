#!/usr/bin/env python3
"""
Search for HYPE in Hyperliquid spot tokens
"""
import requests
import json

API_URL = "https://api.hyperliquid.xyz/info"

def get_spot_meta():
    payload = {"type": "spotMeta"}
    response = requests.post(API_URL, json=payload)
    return response.json()

def main():
    print("=" * 80)
    print("SEARCHING FOR HYPE SPOT MARKET")
    print("=" * 80)
    print()

    spot_meta = get_spot_meta()

    print("📊 All SPOT TOKENS:")
    print()

    if "tokens" in spot_meta:
        tokens = spot_meta["tokens"]
        print(f"Found {len(tokens)} tokens:\n")

        for i, token in enumerate(tokens):
            if isinstance(token, dict):
                name = token.get('name', 'N/A')
                token_id = token.get('index', i)

                # Highlight HYPE-related tokens
                if 'HYPE' in name.upper():
                    print(f"🔍 >>> {token_id}: {name} - {token}")
                elif i < 20 or '@' not in str(name):  # Show first 20 or named tokens
                    print(f"   {token_id}: {name}")
            else:
                print(f"   {i}: {token}")

    print()
    print("-" * 80)
    print()

    # Check spot markets for HYPE
    if "universe" in spot_meta:
        markets = spot_meta["universe"]
        print(f"📊 Searching {len(markets)} spot markets for HYPE...")
        print()

        hype_markets = []
        for market in markets:
            if isinstance(market, dict):
                name = market.get('name', '')
                if 'HYPE' in name.upper():
                    hype_markets.append(market)
                    print(f"🎯 FOUND: {json.dumps(market, indent=2)}")

        if not hype_markets:
            print("❌ NO HYPE SPOT MARKET FOUND!")
            print()
            print("This means:")
            print("  1. HYPE does not have a spot market on Hyperliquid")
            print("  2. You cannot do perp-spot arbitrage for HYPE")
            print("  3. Your bot strategy needs to change")
            print()
            print("Showing some actual spot markets as examples:")
            for market in markets[:5]:
                print(f"   - {market.get('name')}")

    print()
    print("=" * 80)
    print()
    print("💡 RECOMMENDATION:")
    print()
    print("If HYPE has no spot market, you have two options:")
    print()
    print("1. Change to a coin that HAS both perp and spot")
    print("   Example: BTC, ETH, SOL (check which ones have spot)")
    print()
    print("2. Change strategy to perp funding rate arbitrage")
    print("   (This is different from perp-spot arbitrage)")
    print()
    print("=" * 80)

if __name__ == "__main__":
    main()
