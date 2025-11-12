#!/usr/bin/env python3
"""Check current PURR perp-spot BPS"""
import requests

API_URL = "https://api.hyperliquid.xyz/info"

def get_orderbook(coin):
    payload = {"type": "l2Book", "coin": coin}
    return requests.post(API_URL, json=payload).json()

# Get PURR PERP
print("📊 PURR PERP:")
perp = get_orderbook("PURR")
if perp.get("levels"):
    perp_ask = float(perp["levels"][1][0]["px"])
    print(f"   Ask: ${perp_ask:.6f}")
else:
    print("   ❌ No data")
    exit()

# Get PURR SPOT
print("\n📊 PURR SPOT (PURR/USDC):")
spot = get_orderbook("PURR/USDC")
if spot.get("levels"):
    spot_bid = float(spot["levels"][0][0]["px"])
    print(f"   Bid: ${spot_bid:.6f}")
else:
    print("   ❌ No data")
    exit()

# Calculate BPS
bps = ((perp_ask - spot_bid) / spot_bid) * 10000
print(f"\n🎯 PURR BPS: {bps:.2f}")
if bps > 5:
    print(f"   ✅ OPPORTUNITY! Perp premium: {bps:.2f} BPS")
else:
    print(f"   ⚠️  Below threshold (need > 5 BPS)")
