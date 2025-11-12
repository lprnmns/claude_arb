#!/usr/bin/env python3
"""
Simple script to check current BPS between HYPE perp and spot on Hyperliquid
"""
import requests
import json

API_URL = "https://api.hyperliquid.xyz/info"

def get_orderbook(coin):
    """Get L2 orderbook for a coin"""
    payload = {
        "type": "l2Book",
        "coin": coin
    }
    response = requests.post(API_URL, json=payload)
    return response.json()

def calculate_bps(perp_ask, spot_bid):
    """Calculate BPS: ((perp_ask - spot_bid) / spot_bid) * 10000"""
    return ((perp_ask - spot_bid) / spot_bid) * 10000

def main():
    print("=" * 60)
    print("HYPERLIQUID BPS CHECKER - HYPE PERP-SPOT")
    print("=" * 60)
    print()

    # Get PERP orderbook
    print("📊 Fetching HYPE PERP orderbook...")
    try:
        perp_data = get_orderbook("HYPE")
        if perp_data.get("levels"):
            bids = perp_data["levels"][0]  # Bids
            asks = perp_data["levels"][1]  # Asks

            if asks:
                perp_ask_price = float(asks[0]["px"])
                perp_ask_size = float(asks[0]["sz"])
                print(f"   ✅ PERP Best Ask: ${perp_ask_price:.4f} (size: {perp_ask_size:.2f})")
            else:
                print("   ❌ No asks found for PERP")
                return

            if bids:
                perp_bid_price = float(bids[0]["px"])
                perp_bid_size = float(bids[0]["sz"])
                print(f"   ✅ PERP Best Bid: ${perp_bid_price:.4f} (size: {perp_bid_size:.2f})")
        else:
            print("   ❌ No data returned for HYPE")
            return
    except Exception as e:
        print(f"   ❌ Error fetching PERP: {e}")
        return

    print()

    # Try different spot market formats
    spot_symbols = [
        "HYPE-SPOT",
        "HYPE@1",
        "HYPE/USDC",
        "sHYPE",
        "HYPE_SPOT"
    ]

    spot_found = False

    for symbol in spot_symbols:
        print(f"📊 Trying SPOT market: {symbol}...")
        try:
            spot_data = get_orderbook(symbol)

            if spot_data.get("levels"):
                bids = spot_data["levels"][0]
                asks = spot_data["levels"][1]

                if bids and asks:
                    spot_bid_price = float(bids[0]["px"])
                    spot_bid_size = float(bids[0]["sz"])
                    spot_ask_price = float(asks[0]["px"])
                    spot_ask_size = float(asks[0]["sz"])

                    print(f"   ✅ SPOT Best Bid: ${spot_bid_price:.4f} (size: {spot_bid_size:.2f})")
                    print(f"   ✅ SPOT Best Ask: ${spot_ask_price:.4f} (size: {spot_ask_size:.2f})")
                    print()

                    # Calculate BPS
                    bps = calculate_bps(perp_ask_price, spot_bid_price)

                    print("=" * 60)
                    print("📈 BPS CALCULATION RESULTS")
                    print("=" * 60)
                    print(f"Perp Ask:  ${perp_ask_price:.4f}")
                    print(f"Spot Bid:  ${spot_bid_price:.4f}")
                    print(f"Spread:    ${(perp_ask_price - spot_bid_price):.4f}")
                    print()
                    print(f"🎯 BPS: {bps:.2f}")
                    print()

                    if bps > 0:
                        print(f"✅ POSITIVE BPS - Perp trading at PREMIUM")
                        print(f"   Strategy: SHORT PERP @ ${perp_ask_price:.4f}, LONG SPOT @ ${spot_bid_price:.4f}")
                    else:
                        print(f"⚠️  NEGATIVE BPS - Spot trading at PREMIUM")
                        print(f"   (No arbitrage opportunity for perp premium strategy)")

                    print()
                    print("=" * 60)

                    # Also show perp internal spread for comparison
                    perp_spread_bps = ((perp_ask_price - perp_bid_price) / perp_bid_price) * 10000
                    print()
                    print("📊 For comparison:")
                    print(f"   PERP internal spread: {perp_spread_bps:.2f} BPS")
                    print(f"   (This is what your bot might be calculating if both")
                    print(f"    orderbooks are receiving PERP data)")

                    spot_found = True
                    break
            else:
                print(f"   ❌ No data for {symbol}")
        except Exception as e:
            print(f"   ❌ Error: {e}")

    if not spot_found:
        print()
        print("=" * 60)
        print("❌ COULD NOT FIND SPOT MARKET DATA")
        print("=" * 60)
        print()
        print("This confirms the issue: Hyperliquid spot market uses")
        print("a different symbol format that we need to identify.")
        print()
        print("Your bot is likely calculating PERP internal spread")
        print("instead of perp-spot arbitrage BPS.")
        print()

        # Show what the bot is probably calculating
        perp_spread_bps = ((perp_ask_price - perp_bid_price) / perp_bid_price) * 10000
        print(f"PERP internal spread (ask-bid): {perp_spread_bps:.2f} BPS")
        print("^ This matches your bot's negative BPS readings!")

if __name__ == "__main__":
    main()
